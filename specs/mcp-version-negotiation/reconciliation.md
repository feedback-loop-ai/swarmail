# MCP source reconciliation — 2026-10-26

#152 provides version negotiation for 2025-03-26, 2025-06-18 and 2025-11-25,
with batching for 2025-03-26 only. Full revision conformance is not claimed.
The operator's 2026-10-26 '#152 conformance' ruling chooses to retain C1–C6
as documented deviations. Earlier decisions explain the retained contracts;
they do not compel every deviation, particularly C1's HTTP 200 status.
C5 is an example/SHOULD departure, not a mandatory unknown-tool code violation.
0079 and 0080 stay proposed; 2026-07-28 is not supported.

This ledger preserves the previous run's clause audit and source evidence.
Current disposition follows [spec.md](spec.md) and proposed
[0080](../../docs/decisions/0080-mcp-source-reconciliation.md); earlier repair
receipts below are historical, not instructions for further runtime changes.
Source mismatches remain mismatches under operator-directed retention.
The client matrix (#151 wrappers, then the epic exit criteria) determines impact;
version negotiation and local fixtures do not prove actual-client compatibility.

Source: modelcontextprotocol/modelcontextprotocol commit
`dae694f61a31f7a3ec2af3aff709e11f70f61e48` from SOURCE.txt. All 119 current packet
SHA256SUMS entries verify, including README, operator rulings and review-998279d4.
The [spec manifest](spec.md#source-manifest) pins all 18 scoped sources;
the byte manifest below independently records them. The README prior-runs
section is empty; review-998279d4 records the relevant framing finding and
review-95b27901 records the earlier review. Packet paths are evidence locators,
not tracked hyperlinks. No GitHub, CI, journal, sibling or upstream read was used.

## Reading the ledger

References are relative to each revision's supplied snapshot, with literal
pinned line numbers. Every normative-keyword line has a row, including repeated
MUSTs, conditional obligations and MAYs. Governing keywords cover their multiline
continuations/lists; compound security/operation duties get separate child rows.
Non-keyword shapes are audited by schema symbol and supplemental rows. Owner
abbreviations name src/mcp.rs, src/api.rs, src/stdio.rs and src/config.rs.
Rule keys expand applicability/reasons below; W keys expand executable proofs.
No clause is waived merely for a fixed catalogue or loopback listener.

## Executable witnesses

| Ref | Named proofs |
| --- | --- |
| W1 | tests/mcp_revisions.rs: revision_negotiation_matrix, revision_headers_and_isolation, revision_rpc_edges, revision_smtp_visibility. Real HTTP/SMTP and binary: all selection rows, omitted binary version, required server metadata, excluded 2026 revision, isolated clients. |
| W2 | tests/mcp_revisions.rs: revision_transport_methods, revision_response_disposition, revision_served_contract. Real status/media/body, all methods and header shapes, Origin precedence, served OpenAPI. |
| W3 | tests/stdio.rs: revision_exchange_fences_and_headers_follow_the_result, failed_modern_negotiation_never_releases_queued_effects_as_legacy, initialized_failure_stays_fenced_and_modern_arrays_never_dial, incomplete_initialize_results_cannot_enable_operations, lost_and_truncated_initialization_stay_failed_until_explicit_retry; tests/mcp_revisions.rs bridge_revision_lifecycle and bridge_modern_wait_allows_seed. Real TCP failure peers, fences/recovery/headers, earlier completions and causal next-exchange witnesses. |
| W4 | revision_response_disposition and revision_rpc_edges; tests/stdio.rs a_server_error_object_is_forwarded_verbatim (peer error lacks required message), a_multiline_reply_is_written_as_one_line; src/stdio.rs parser tests. Current C1/C2/C6 behavior, explicitly not proof of conformance. |
| W5 | modern_batches_refuse_before_effects; src/mcp.rs revision_admission_has_no_transient_mutations_or_waiters; tests/mcp.rs a_batched_wait_is_satisfied_by_its_batched_seed; src/mcp/batch_tests.rs and stdio batch/ID cases. Transient counters plus live March controls. |
| W6 | src/api.rs origin_spellings_of_the_bound_port and the_off_loopback_origin_must_name_the_host_authority, whole-plane suites; revision_transport_methods explicitly checks every method's invalid-Origin 403. Default config and accepted 0030/0043 define admission, not OAuth identity. |
| W7 | revision_tool_annotations, revision_tool_schemas_and_text_safety; tests/mcp.rs and src/mcp.rs staging/closed-shape/null/bounds/default/chaos/seed/results suites. Twelve schemas/names/hints, malicious mail as inert typed text, raw omission, no icons/tasks/#153 outputs. Finite vocabulary audit below complements tests. |
| W8 | Full tests/stdio.rs suite, especially bad_lines_are_answered_locally_and_the_next_request_still_runs, crlf_blank_and_unterminated_lines_frame_correctly, every_reply_is_one_complete_write_then_a_flush, eof_drains_every_admitted_exchange, a_stdin_error_drains_admitted_replies_then_exits_one, a_stdout_failure_exits_one_without_waiting_for_held_work, short_http_bodies_fail_correlated_slots_and_the_bridge_recovers, a_silent_server_answers_as_a_read_timeout, a_wait_longer_than_ten_seconds_gets_its_result; owner parser/deadline tests. |
| W9 | src/mcp.rs tool_rate_uses_exact_monotonic_windows and exhausted_tool_rate_refuses_before_effects_and_keeps_control_available. Exact monotonic boundary/refill; zero mutation/subscription counters with live controls; silence and control availability. HTTP/binary suites execute admitted calls through the router. They do not prove shared-budget exhaustion on the wire; the prior review's rate-plumbing finding remains recorded, not discharged by this wording task. |

## Exact conflict register

Each row records retained behavior, not a new repair commission. Source references
are to the supplied revision snapshots; the clause/schema rows below give literal
line numbers. “Retained deviation” in those rows means an unmet source obligation
kept by operator choice, not permission still awaited to keep current behavior.

| ID | Source obligation / strength / actor | 2025-03-26 observation | 2025-06-18 observation | 2025-11-25 observation | Contract attribution and operator disposition | Witness / compatibility cost |
| --- | --- | --- | --- | --- | --- | --- |
| C1 | All three Transports / Sending Messages item 4: server MUST accept responses with empty HTTP 202 or reject with an HTTP error; an RPC error body is MAY. | Unsolicited success/error objects get HTTP 200, -32600 `missing method`, carrying valid IDs. Response-only batches keep per-member errors; mixed response/request batches keep each member's reply despite being outside March's separate batch kinds. | Same single-object 200/-32600 behavior. Every array instead gets one pre-dispatch -32600/null refusal, with no member effects. | Same as June, including whole-array refusal. | 0029 §2 pins the RPC envelope/carried ID; 0044 §1 explicitly pins 200 for non-objects; 0047 §3 pins notification classification. None compels 200 for response-shaped objects. The independent review's HTTP-400-with-existing-envelope observation corrects the old over-attribution. The operator explicitly retains 200/-32600; no status/empty-body repair is commissioned or awaiting acceptance here. | W2/W4/W5. Rejection uses a success HTTP status contrary to item 4; clients expecting 202 or HTTP error may misclassify it. No pending-request service is claimed. |
| C2 | Basic / Requests: request sender MUST use non-null string/integer IDs, unused within the session; response sender MUST echo them. RequestId TypeScript is string/number, generated JSON is string/integer. March/June error ID is required and RequestId-shaped; November permits absence, not explicit null. | Receiver admits string/number/null, including fractional values; Boolean/object/array IDs refuse -32600/null. Owned null/fractional-ID successes and null-ID errors retain schema mismatches. Admitted batches preserve duplicate/null slots; no history cache. | Same single-ID admission and owned-output mismatches; arrays refuse as a whole instead of preserving member slots. Error schema still requires a RequestId-shaped ID. | Same as June admission and array refusal. Errors still emit explicit null rather than taking the schema's absent-ID alternative. | 0047 §3's accepted 2026-10-24 amendment retains the supplied domain/values; 0029/0044 pin null parse/invalid errors. Operator retains C2 without tightening, caching or renumbering. Sender integer/non-null/uniqueness duties do not themselves mandate a receiver cache or refusal; owned output duties remain distinct. | W1/W4/W5. Tolerant input does not make invalid senders conformant; owned outputs may fail schema validation and reused IDs can confuse client correlation. |
| C3 | All three Transports / Sending Messages item 5: HTTP client MUST support JSON and SSE. Session Management: if a peer assigns a session ID, client MUST propagate it and restart initialization after session 404; termination DELETE is SHOULD. November additionally requires secure session-ID handling. | Bridge sends both JSON/SSE Accept tokens but consumes identity JSON/empty bodies only. External SSE and applicable session-ID/404 duties are unsupported. Paired server is stateless JSON; GET is 405 with valid headers. | Same as March; negotiation/header support does not add SSE or session handling. | Same as June, including absence of external session lifecycle/secure session-ID handling. | 0039 §§2/6 and 0029 §4 describe parser/forwarding. Operator retains C3; this task adds no streaming/session machinery. A paired server assigning no sessions leaves its conditional duties inapplicable, but cannot discharge the general HTTP client's JSON/SSE MUST or duties when an external peer assigns a session. | W2/W3/W8 and owner parser tests. Works with the paired identity-JSON server; no general Streamable HTTP client claim, SSE peer support or stateful-peer lifecycle guarantee. |
| C4 | JSONRPC schemas require jsonrpc 2.0 and object params where supplied; Lifecycle requires the initialization sender's first initialize with version/capabilities/clientInfo, then initialized. March separately MUST NOT batch initialize. Sender obligations are distinct from receiver admission. | Missing/wrong jsonrpc, omitted client fields and currently tolerated invalid envelopes/params remain admitted/forwarded where existing dispatch permits. Legacy first-line operations and batched initialize remain tolerated; batched initialize returns March without transitioning bridge state. | Same envelope/legacy-omission tolerance for single messages; modern bridge initialization fences remain. Arrays refuse before dispatch, including batched initialize. | Same as June, including whole-array refusal. | 0044 §1 adds no version check; 0047 §3 preserves params posture; 0039 §§1/2/4 describe independent value forwarding. Proposed 0079 records negotiation/legacy compatibility. Operator retains these observations; no general validator or stricter admission is commissioned. | W1/W3/W5/W7/W8. Compatibility fixtures are not conforming senders; forwarding invalid input also affects the bridge's HTTP sender role. This does not promise acceptance of every invalid tool argument: staged/closed argument refusals remain. |
| C5 | All three Tools / Error Handling show unknown-tool -32602 as an example, not a mandated code. CallToolResult recommends isError for tool-origin errors (SHOULD). November adds explanation distinguishing validation/execution feedback from protocol errors. | Unknown tools use -32601; staged invalid arguments use -32602. Wait's malformed selector/regex uses -32602; supplied timeout refusal is isError true; elapsed wait is a normal isError false result. | Same as March; arrays refuse before any member's tool dispatch. | Same as June; the additional validation/execution-versus-protocol explanation does not change the retained split. | 0029 §2 explicitly pins the taxonomy. Operator retains C5 as an example/SHOULD departure with actionable field messages, not an invented MUST-code violation. | W1/W7 and existing MCP wait/error tests. Clients following the example or expecting all tool-origin failures in isError must handle the deliberate envelope/result split; valid-ID output shapes are otherwise retained. |
| C6 | All three Transports / stdio: server MUST NOT write anything to stdout that is not a valid MCP message. Complete JSON alone does not satisfy the message schemas. | For reply-bearing exchanges the bridge can forward complete JSON values with missing required jsonrpc/message/ID fields or simultaneous result/error. Initialize metadata validation controls state installation only; malformed complete replies can still be written. | Same as March for single replies; modern arrays refuse locally before dialing. | Same as June. | 0039 §§2/6 and 0047's dated ID amendment describe complete reply forwarding. Operator retains C6; no general output validator or reply replacement is commissioned. Syntactically invalid/truncated JSON and invalid UTF-8 still fail parsing; this row does not claim they are forwarded. | W3/W4/W8. Malformed peers can cause invalid MCP stdout even though framing is one complete JSON line; strict clients may reject it. Metadata validation is not full-envelope validation. |

The operator's choice resolves this run's disposition, not the source differences.
Preserve 0001–0005 and current runtime. 0079/0080 stay proposed. Decision
acceptance, #151 baseline, sealing and merge remain separate operator acts.
No current instruction to repair C1–C6 follows from the historical ledger.

## Rule explanations

| Key | Applicability and reason |
| --- | --- |
| base | Base/lifecycle implemented; mandatory conflicts prevent full conformance. |
| format | Owned responses use 2.0 and exclusive result/error. Accepted invalid-input/peer forwarding and null/fractional outputs remain nonconforming. |
| ids | Null/fractional/reused peer IDs violate sender rules, without mandating a receiver cache. Owned null/fractional-ID outputs also conflict with generated schemas; November permits absent error ID, not null. |
| response | Owned success/error are exclusive, results objects, codes integers, messages strings; matching IDs preserved. C2 separately accounts for invalid IDs. |
| notification | No ID on conforming notifications; receiver executes silently, HTTP acknowledgment empty 202. |
| batch | March bounded concurrent batches accepted; modern arrays refused before effects. March batched initialize remains invalid-client compatibility. |
| auth | Local development trust: loopback default, Host/Origin admission, operator-controlled exposure. No OAuth/user authentication; explicit SHOULD departure under 0030/0043. Stdio URL uses environment/configuration. |
| meta | No owned _meta; peer extension metadata opaque, never authorization. Grammar constrains producers, not a mandatory receiver refusal. |
| dialect | Default 2020-12 owned schemas use finite audited vocabulary and staged validation. No remote/schema-valued input consumer exists. No general schema validator or additional dialect support claimed. |
| icons | No owned icons and no peer-icon fetching/rendering or MIME/redirect/URI interpretation. |
| init-client | Conforming sender supplies version/capabilities/clientInfo then initialized. Legacy first-line operations, omitted fields and batched initialize are invalid-client compatibility. |
| init-server | Server emits required version, capabilities and name/version. Bridge now checks common required result fields before state installation; forwarding C6 unchanged. |
| pre-client | Initialize fences admission, ping allowed, earlier completions serviced. Legacy independent lines retained by 0039 §4. |
| pre-server | No server-issued requests/logging/notifications, before or after initialization. |
| version | Exactly three revisions; supported echo, unsupported selects November (including 2026-07-28). Unsupported result installs no state; HTTP connection closes; explicit retry recovers. |
| operation | Immutable exchange revision/header; tools-only server output, no client-feature/task use. HTTP has no history/session registry. |
| shutdown | EOF drains then exits; HTTP uses Connection: close. Application launching children owns TERM/KILL escalation. |
| timeout | 5s dial, derived 65s maximum. No progress reset (MAY); no per-request configuration or cancellation notification, retaining 0029 §4/0039 §6 derived budget/no retry. Socket close promises no rollback. |
| errors | Unsupported, malformed, lost or truncated results never install state; explicit retry recovers. Error kinds remain distinct. |
| utf8 | UTF-8 JSON serialization; invalid UTF-8 input/reply refuses. Byte encoding is separate from message shape. |
| stdio-support | Shipped bridge reads stdin/writes stdout; actual model-client receipts remain owed. |
| framing | One compact value plus LF, no literal embedded newline; single writer/flush; CRLF/blank/final unterminated lines recover. |
| logging | MCP diagnostics use stderr. No protocol failure inferred from stderr; application decides capture policy. |
| stdout | No banners/logs; accepted verbatim peer JSON can lack required fields. A JSON line is not necessarily a valid MCP message. |
| stdin | Application produces stdin. Bridge rejects parse/UTF-8 failures, tolerates other invalid client envelopes per accepted contracts. |
| endpoint | Single /mcp supports POST and GET via allowed 405 alternative; no SSE service. |
| origin | Whole-plane Host/Origin admission precedes method/body rules; invalid present Origin returns 403, including November. Omission allowed for non-browser clients. |
| post | Each exchange one POST, application/json, both mandatory Accept tokens; no splitting or retry. |
| post-body | Modern single message; March request/notification batches and separate response batches. Invalid peers retain accepted taxonomy. |
| accepted-input | Notifications empty 202. Valid unsolicited responses instead get 200/-32600 missing method. March response-only/mixed cases witnessed separately. |
| json-or-sse | Server chooses application/json. Bridge advertises JSON/SSE but parses identity JSON/empty only; fixed peer does not discharge general client MUST. |
| sse | No server SSE/event/retry/stream created, so conditional server duties have false antecedent. External SSE support is absent, not excused. |
| get | GET valid/absent version 405; invalid version 400; invalid Origin 403. Bridge opens no listening stream. |
| connections | No SSE connections, broadcast or cross-stream delivery. Parallel POSTs are not SSE streams. |
| resume | No event IDs, replay history or Last-Event-ID service; external resumable client support absent. |
| session | Paired server assigns/requires/expires no session. DELETE 405. Bridge cannot propagate external session IDs/restart on session 404; 0039 §§2/6 preserve parser/forwarding. |
| header | Supported single header selects revision; absent March. Invalid/duplicate/combined/non-text/unsupported ->400 after POST body/parse. Subsequent bridge headers follow result. Initialize body/header difference need not refuse. |
| old-http | Deprecated two-endpoint transport and optional migration procedure not selected/advertised. |
| custom | Only standard stdio/Streamable HTTP, no third custom transport. |
| ui | Headless server exposes discovery/hints, no model UI. Applications own confirmation and display. Hints never grant authorization. |
| capability | Exactly tools:{}; no listChanged, resources, prompts or task capability. |
| list-change | Catalogue fixed; listChanged not declared, no notification owed. |
| annotations | Hints never authorize; bridge does not interpret them. Six reads true; six mutations false/destructive true; other defaults retained. |
| input-schema | Twelve non-null object schemas; no-argument schemas close additionalProperties. Finite default-2020-12 vocabulary audited, not arbitrary remote validation. |
| names | Twelve unique 1–128 ASCII permitted-character names; dispatch case sensitive. |
| resource | TextContent only; no resource_link/embedded resource, resource capability or URL fetch. |
| structured | No outputSchema/structuredContent; #153 separate. Conditional structured validation/text fallback not triggered. |
| client-tools | External client owns confirmation/display/result-to-model validation/audit logging; bridge has timeout and no model/UI execution. |
| server-tools | Separate input, access-admission, actual rate-budget and output-context evidence; no concurrency/serialization-only exemption. |

## Normative prose ledger

### 2025-03-26

| Source / heading | Level / actor | Disposition; owner; witness | Literal requirement cue |
| --- | --- | --- | --- |
| `spec/basic/index.mdx:14` Overview | MUST; both | retained deviation C1/C2/C3/C4/C6 (base); mcp/stdio; W1/W3/W4 | All implementations MUST support the base protocol and lifecycle management |
| `spec/basic/index.mdx:15` Overview | MAY; both | conforms negotiated operations; C4 invalid forwarding (operation); mcp/stdio; W1/W3/W5/W7 | components. Other components MAY be implemented based on the specific needs of the |
| `spec/basic/index.mdx:24` Messages | MUST; senders | retained deviation C2/C4/C6 (format); mcp/stdio; W1/W3/W4 | All messages between MCP clients and servers MUST follow the |
| `spec/basic/index.mdx:43` Requests | MUST; request/response sender | retained deviation C2; server receiver tolerance retained (ids); mcp/stdio; W1/W4 | - Requests MUST include a string or integer ID. |
| `spec/basic/index.mdx:44` Requests | MUST NOT; request/response sender | retained deviation C2; server receiver tolerance retained (ids); mcp/stdio; W1/W4 | - Unlike base JSON-RPC, the ID MUST NOT be `null`. |
| `spec/basic/index.mdx:45` Requests | MUST NOT; request/response sender | retained deviation C2; server receiver tolerance retained (ids); mcp/stdio; W1/W4 | - The request ID MUST NOT have been previously used by the requestor within the same |
| `spec/basic/index.mdx:67` Responses | MUST; request/response sender | retained deviation C2; server receiver tolerance retained (ids); mcp/stdio; W1/W4 | - Responses MUST include the same ID as the request they correspond to. |
| `spec/basic/index.mdx:69` Responses | MUST/MUST NOT; response sender | conforms for owned valid-ID responses; C6 for forwarded replies (response); mcp/stdio; W1/W4 | errors. Either a `result` or an `error` MUST be set. A response MUST NOT |
| `spec/basic/index.mdx:71` Responses | MAY/MUST; response sender | conforms for owned valid-ID responses; C6 for forwarded replies (response); mcp/stdio; W1/W4 | - Results MAY follow any JSON object structure, while errors MUST include an |
| `spec/basic/index.mdx:73` Responses | MUST; response sender | conforms for owned valid-ID responses; C6 for forwarded replies (response); mcp/stdio; W1/W4 | - Error codes MUST be integers. |
| `spec/basic/index.mdx:78` Notifications | MUST NOT; sender/receiver | conforms for valid notifications; C4 for invalid-input forwarding (notification); mcp/stdio; W1/W3 | The receiver MUST NOT send a response. |
| `spec/basic/index.mdx:90` Notifications | MUST NOT; sender/receiver | conforms for valid notifications; C4 for invalid-input forwarding (notification); mcp/stdio; W1/W3 | - Notifications MUST NOT include an ID. |
| `spec/basic/index.mdx:96` Batching | MAY; receiver; optional sender | conforms (batch); mcp/stdio; W1/W5 | by sending them in an array. MCP implementations MAY support sending JSON-RPC |
| `spec/basic/index.mdx:97` Batching | MUST; receiver; optional sender | conforms (batch); mcp/stdio; W1/W5 | batches, but MUST support receiving JSON-RPC batches. |
| `spec/basic/index.mdx:102` Auth | SHOULD; HTTP implementation; stdio client | justified SHOULD departure (auth); api/config; W6 | Implementations using an HTTP-based transport SHOULD conform to this specification, |
| `spec/basic/index.mdx:103` Auth | SHOULD NOT; HTTP implementation; stdio client | justified SHOULD departure (auth); api/config; W6 | whereas implementations using STDIO transport SHOULD NOT follow this specification, |
| `spec/basic/index.mdx:106` Auth | MAY; HTTP implementation; stdio client | justified SHOULD departure (auth); api/config; W6 | Additionally, clients and servers MAY negotiate their own custom authentication and |
| `spec/basic/lifecycle.mdx:38` Initialization | MUST; initialization client | C4 for compatibility; conforms valid modern flow (init-client); stdio/mcp; W1/W3 | The initialization phase MUST be the first interaction between client and server. |
| `spec/basic/lifecycle.mdx:45` Initialization | MUST; initialization client | C4 for compatibility; conforms valid modern flow (init-client); stdio/mcp; W1/W3 | The client MUST initiate this phase by sending an `initialize` request containing: |
| `spec/basic/lifecycle.mdx:72` Initialization | MUST NOT; initialization client | C4 for compatibility; conforms valid modern flow (init-client); stdio/mcp; W1/W3 | The initialize request MUST NOT be part of a JSON-RPC |
| `spec/basic/lifecycle.mdx:78` Initialization | MUST; server; bridge state owner | conforms; required-result installation repaired (init-server); mcp/stdio; W1/W3 | The server MUST respond with its own capabilities and information: |
| `spec/basic/lifecycle.mdx:108` Initialization | MUST; initialization client | C4 for compatibility; conforms valid modern flow (init-client); stdio/mcp; W1/W3 | After successful initialization, the client MUST send an `initialized` notification |
| `spec/basic/lifecycle.mdx:118` Initialization | SHOULD NOT; client | modern fence conforms; legacy SHOULD departure (pre-client); stdio; W3 | - The client SHOULD NOT send requests other than |
| `spec/basic/lifecycle.mdx:121` Initialization | SHOULD NOT; server | conforms (pre-server); mcp; W3 | - The server SHOULD NOT send requests other than |
| `spec/basic/lifecycle.mdx:128` Version Negotiation | MUST; initialization sender/server | conforms valid single initialize; C4 invalid fixtures (version); mcp/stdio; W1/W3 | In the `initialize` request, the client MUST send a protocol version it supports. |
| `spec/basic/lifecycle.mdx:129` Version Negotiation | SHOULD; initialization sender/server | conforms valid single initialize; C4 invalid fixtures (version); mcp/stdio; W1/W3 | This SHOULD be the _latest_ version supported by the client. |
| `spec/basic/lifecycle.mdx:131` Version Negotiation | MUST; initialization sender/server | conforms valid single initialize; C4 invalid fixtures (version); mcp/stdio; W1/W3 | If the server supports the requested protocol version, it MUST respond with the same |
| `spec/basic/lifecycle.mdx:132` Version Negotiation | MUST; initialization sender/server | conforms valid single initialize; C4 invalid fixtures (version); mcp/stdio; W1/W3 | version. Otherwise, the server MUST respond with another protocol version it |
| `spec/basic/lifecycle.mdx:133` Version Negotiation | SHOULD; initialization sender/server | conforms valid single initialize; C4 invalid fixtures (version); mcp/stdio; W1/W3 | supports. This SHOULD be the _latest_ version supported by the server. |
| `spec/basic/lifecycle.mdx:135` Version Negotiation | SHOULD; initialization sender/server | conforms valid single initialize; C4 invalid fixtures (version); mcp/stdio; W1/W3 | If the client does not support the version in the server's response, it SHOULD |
| `spec/basic/lifecycle.mdx:168` Operation | SHOULD; both | conforms negotiated operations; C4 invalid forwarding (operation); mcp/stdio; W1/W3/W5/W7 | Both parties SHOULD: |
| `spec/basic/lifecycle.mdx:181` stdio | SHOULD; stdio launcher; optional server | bridge conforms; launcher inapplicable (shutdown); stdio; W8 | For the stdio transport, the client SHOULD initiate |
| `spec/basic/lifecycle.mdx:189` stdio | MAY; stdio launcher; optional server | bridge conforms; launcher inapplicable (shutdown); stdio; W8 | The server MAY initiate shutdown by closing its output stream to the client and |
| `spec/basic/lifecycle.mdx:199` Timeouts | SHOULD; sender/middleware | conforms with SHOULD departures (timeout); stdio; W8 | Implementations SHOULD establish timeouts for all sent requests, to prevent hung |
| `spec/basic/lifecycle.mdx:201` Timeouts | SHOULD; sender/middleware | conforms with SHOULD departures (timeout); stdio; W8 | response within the timeout period, the sender SHOULD issue a [cancellation |
| `spec/basic/lifecycle.mdx:205` Timeouts | SHOULD; sender/middleware | conforms with SHOULD departures (timeout); stdio; W8 | SDKs and other middleware SHOULD allow these timeouts to be configured on a |
| `spec/basic/lifecycle.mdx:208` Timeouts | MAY; sender/middleware | conforms with SHOULD departures (timeout); stdio; W8 | Implementations MAY choose to reset the timeout clock when receiving a [progress |
| `spec/basic/lifecycle.mdx:210` Timeouts | SHOULD; sender/middleware | conforms with SHOULD departures (timeout); stdio; W8 | implies that work is actually happening. However, implementations SHOULD always |
| `spec/basic/lifecycle.mdx:216` Error Handling | SHOULD; implementation | conforms (errors); mcp/stdio; W1/W3/W8 | Implementations SHOULD be prepared to handle these error cases: |
| `spec/basic/transports.mdx:5` Overview | MUST; transport senders | conforms owned bytes; semantic C6 separate (utf8); mcp/stdio; W8 | MCP uses JSON-RPC to encode messages. JSON-RPC messages MUST be UTF-8 encoded. |
| `spec/basic/transports.mdx:13` Overview | SHOULD; client | conforms (stdio-support); stdio; W1/W8 | Clients SHOULD support stdio whenever possible. |
| `spec/basic/transports.mdx:28` stdio | MUST NOT; stdio senders | conforms (framing); stdio; W8 | - Messages are delimited by newlines, and MUST NOT contain embedded newlines. |
| `spec/basic/transports.mdx:29` stdio | MAY; server; stderr consumer | conforms (logging); stdio/main; W8 | - The server MAY write UTF-8 strings to its standard error (`stderr`) for logging |
| `spec/basic/transports.mdx:30` stdio | MAY; server; stderr consumer | conforms (logging); stdio/main; W8 | purposes. Clients MAY capture, forward, or ignore this logging. |
| `spec/basic/transports.mdx:31` stdio | MUST NOT; stdio server | retained deviation C2/C6; owned framing conforms (stdout); stdio; W4/W8 | - The server MUST NOT write anything to its `stdout` that is not a valid MCP message. |
| `spec/basic/transports.mdx:32` stdio | MUST NOT; external stdio client | producer inapplicable; bridge forwarding C4 (stdin); stdio; W1/W8 | - The client MUST NOT write anything to the server's `stdin` that is not a valid MCP |
| `spec/basic/transports.mdx:68` Streamable HTTP | MUST; HTTP server | conforms (endpoint); api; W2 | The server MUST provide a single HTTP endpoint path (hereafter referred to as the |
| `spec/basic/transports.mdx:76` Security Warning | MUST; HTTP server | conforms (origin); api; W2/W6 | 1. Servers MUST validate the `Origin` header on all incoming connections to prevent DNS rebinding attacks |
| `spec/basic/transports.mdx:77` Security Warning | SHOULD; HTTP implementation; stdio client | justified SHOULD departure (auth); api/config; W6 | 2. When running locally, servers SHOULD bind only to localhost (127.0.0.1) rather than all network interfaces (0.0.0.0) |
| `spec/basic/transports.mdx:78` Security Warning | SHOULD; HTTP implementation; stdio client | justified SHOULD departure (auth); api/config; W6 | 3. Servers SHOULD implement proper authentication for all connections |
| `spec/basic/transports.mdx:84` Sending Messages to the Server | MUST; HTTP client | conforms valid input; C4 invalid forwarding (post); stdio; W3 | Every JSON-RPC message sent from the client MUST be a new HTTP POST request to the |
| `spec/basic/transports.mdx:87` Sending Messages to the Server | MUST; HTTP client | conforms valid input; C4 invalid forwarding (post); stdio; W3 | 1. The client MUST use HTTP POST to send JSON-RPC messages to the MCP endpoint. |
| `spec/basic/transports.mdx:88` Sending Messages to the Server | MUST; HTTP client | conforms valid input; C4 invalid forwarding (post); stdio; W3 | 2. The client MUST include an `Accept` header, listing both `application/json` and |
| `spec/basic/transports.mdx:90` Sending Messages to the Server | MUST; HTTP client | conforms valid input; C4 invalid forwarding (post); stdio; W3 | 3. The body of the POST request MUST be one of the following: |
| `spec/basic/transports.mdx:98` Sending Messages to the Server | MUST; HTTP server | notifications conform; responses retained deviation C1 (accepted-input); mcp; W4 | - If the server accepts the input, the server MUST return HTTP status code 202 |
| `spec/basic/transports.mdx:100` Sending Messages to the Server | MUST; HTTP server | notifications conform; responses retained deviation C1 (accepted-input); mcp; W4 | - If the server cannot accept the input, it MUST return an HTTP error status code |
| `spec/basic/transports.mdx:101` Sending Messages to the Server | MAY; HTTP server | notifications conform; responses retained deviation C1 (accepted-input); mcp; W4 | (e.g., 400 Bad Request). The HTTP response body MAY comprise a JSON-RPC _error |
| `spec/basic/transports.mdx:103` Sending Messages to the Server | MUST; HTTP server/client | JSON server conforms; client retained deviation C3 (json-or-sse); mcp/stdio; W2/W3 | 5. If the input contains any number of JSON-RPC _requests_, the server MUST either |
| `spec/basic/transports.mdx:105` Sending Messages to the Server | MUST; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | `Content-Type: application/json`, to return one JSON object. The client MUST |
| `spec/basic/transports.mdx:108` Sending Messages to the Server | SHOULD; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - The SSE stream SHOULD eventually include one JSON-RPC _response_ per each |
| `spec/basic/transports.mdx:109` Sending Messages to the Server | MAY; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | JSON-RPC _request_ sent in the POST body. These _responses_ MAY be |
| `spec/basic/transports.mdx:111` Sending Messages to the Server | MAY; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - The server MAY send JSON-RPC _requests_ and _notifications_ before sending a |
| `spec/basic/transports.mdx:112` Sending Messages to the Server | SHOULD; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | JSON-RPC _response_. These messages SHOULD relate to the originating client |
| `spec/basic/transports.mdx:113` Sending Messages to the Server | MAY; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | _request_. These _requests_ and _notifications_ MAY be |
| `spec/basic/transports.mdx:115` Sending Messages to the Server | SHOULD NOT; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - The server SHOULD NOT close the SSE stream before sending a JSON-RPC _response_ |
| `spec/basic/transports.mdx:118` Sending Messages to the Server | SHOULD; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - After all JSON-RPC _responses_ have been sent, the server SHOULD close the SSE |
| `spec/basic/transports.mdx:120` Sending Messages to the Server | MAY; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - Disconnection MAY occur at any time (e.g., due to network conditions). |
| `spec/basic/transports.mdx:122` Sending Messages to the Server | SHOULD NOT; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - Disconnection SHOULD NOT be interpreted as the client cancelling its request. |
| `spec/basic/transports.mdx:123` Sending Messages to the Server | SHOULD; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - To cancel, the client SHOULD explicitly send an MCP `CancelledNotification`. |
| `spec/basic/transports.mdx:124` Sending Messages to the Server | MAY; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - To avoid message loss due to disconnection, the server MAY make the stream |
| `spec/basic/transports.mdx:129` Listening for Messages from the Server | MAY; optional client; server | server conforms; optional client unused (get); api/stdio; W2 | 1. The client MAY issue an HTTP GET to the MCP endpoint. This can be used to open an |
| `spec/basic/transports.mdx:132` Listening for Messages from the Server | MUST; optional client; server | server conforms; optional client unused (get); api/stdio; W2 | 2. The client MUST include an `Accept` header, listing `text/event-stream` as a |
| `spec/basic/transports.mdx:134` Listening for Messages from the Server | MUST; optional client; server | server conforms; optional client unused (get); api/stdio; W2 | 3. The server MUST either return `Content-Type: text/event-stream` in response to |
| `spec/basic/transports.mdx:138` Listening for Messages from the Server | MAY; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - The server MAY send JSON-RPC _requests_ and _notifications_ on the stream. These |
| `spec/basic/transports.mdx:139` Listening for Messages from the Server | MAY; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | _requests_ and _notifications_ MAY be |
| `spec/basic/transports.mdx:141` Listening for Messages from the Server | SHOULD; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - These messages SHOULD be unrelated to any concurrently-running JSON-RPC |
| `spec/basic/transports.mdx:143` Listening for Messages from the Server | MUST NOT; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - The server MUST NOT send a JSON-RPC _response_ on the stream unless |
| `spec/basic/transports.mdx:146` Listening for Messages from the Server | MAY; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - The server MAY close the SSE stream at any time. |
| `spec/basic/transports.mdx:147` Listening for Messages from the Server | MAY; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - The client MAY close the SSE stream at any time. |
| `spec/basic/transports.mdx:151` Multiple Connections | MAY; SSE sender/receiver | inapplicable with reason (connections); mcp; W2 | 1. The client MAY remain connected to multiple SSE streams simultaneously. |
| `spec/basic/transports.mdx:152` Multiple Connections | MUST; SSE sender/receiver | inapplicable with reason (connections); mcp; W2 | 2. The server MUST send each of its JSON-RPC messages on only one of the connected |
| `spec/basic/transports.mdx:153` Multiple Connections | MUST NOT; SSE sender/receiver | inapplicable with reason (connections); mcp; W2 | streams; that is, it MUST NOT broadcast the same message across multiple streams. |
| `spec/basic/transports.mdx:154` Multiple Connections | MAY; SSE sender/receiver | inapplicable with reason (connections); mcp; W2 | - The risk of message loss MAY be mitigated by making the stream |
| `spec/basic/transports.mdx:162` Resumability and Redelivery | MAY; SSE sender/receiver | server inapplicable; general client C3 (resume); mcp/stdio; W2 | 1. Servers MAY attach an `id` field to their SSE events, as described in the |
| `spec/basic/transports.mdx:164` Resumability and Redelivery | MUST; SSE sender/receiver | server inapplicable; general client C3 (resume); mcp/stdio; W2 | - If present, the ID MUST be globally unique across all streams within that |
| `spec/basic/transports.mdx:167` Resumability and Redelivery | SHOULD; SSE sender/receiver | server inapplicable; general client C3 (resume); mcp/stdio; W2 | 2. If the client wishes to resume after a broken connection, it SHOULD issue an HTTP |
| `spec/basic/transports.mdx:171` Resumability and Redelivery | MAY; SSE sender/receiver | server inapplicable; general client C3 (resume); mcp/stdio; W2 | - The server MAY use this header to replay messages that would have been sent |
| `spec/basic/transports.mdx:174` Resumability and Redelivery | MUST NOT; SSE sender/receiver | server inapplicable; general client C3 (resume); mcp/stdio; W2 | - The server MUST NOT replay messages that would have been delivered on a |
| `spec/basic/transports.mdx:186` Session Management | MAY; optional stateful server; conditional client | paired server inapplicable; external client C3 (session); mcp/stdio; W1/W2/W3 | 1. A server using the Streamable HTTP transport MAY assign a session ID at |
| `spec/basic/transports.mdx:189` Session Management | SHOULD; optional stateful server; conditional client | paired server inapplicable; external client C3 (session); mcp/stdio; W1/W2/W3 | - The session ID SHOULD be globally unique and cryptographically secure (e.g., a |
| `spec/basic/transports.mdx:191` Session Management | MUST; optional stateful server; conditional client | paired server inapplicable; external client C3 (session); mcp/stdio; W1/W2/W3 | - The session ID MUST only contain visible ASCII characters (ranging from 0x21 to |
| `spec/basic/transports.mdx:194` Session Management | MUST; optional stateful server; conditional client | paired server inapplicable; external client C3 (session); mcp/stdio; W1/W2/W3 | the Streamable HTTP transport MUST include it in the `Mcp-Session-Id` header on |
| `spec/basic/transports.mdx:196` Session Management | SHOULD; optional stateful server; conditional client | paired server inapplicable; external client C3 (session); mcp/stdio; W1/W2/W3 | - Servers that require a session ID SHOULD respond to requests without an |
| `spec/basic/transports.mdx:198` Session Management | MAY/MUST; optional stateful server; conditional client | paired server inapplicable; external client C3 (session); mcp/stdio; W1/W2/W3 | 3. The server MAY terminate the session at any time, after which it MUST respond |
| `spec/basic/transports.mdx:201` Session Management | MUST; optional stateful server; conditional client | paired server inapplicable; external client C3 (session); mcp/stdio; W1/W2/W3 | `Mcp-Session-Id`, it MUST start a new session by sending a new `InitializeRequest` |
| `spec/basic/transports.mdx:204` Session Management | SHOULD; optional stateful server; conditional client | paired server inapplicable; external client C3 (session); mcp/stdio; W1/W2/W3 | the client application) SHOULD send an HTTP DELETE to the MCP endpoint with the |
| `spec/basic/transports.mdx:206` Session Management | MAY; optional stateful server; conditional client | paired server inapplicable; external client C3 (session); mcp/stdio; W1/W2/W3 | - The server MAY respond to this request with HTTP 405 Method Not Allowed, |
| `spec/basic/transports.mdx:281` Custom Transports | MAY; optional custom transport implementer | inapplicable with reason (custom); mcp/stdio; W1/W2 | Clients and servers MAY implement additional custom transport mechanisms to suit |
| `spec/basic/transports.mdx:285` Custom Transports | MUST; optional custom transport implementer | inapplicable with reason (custom); mcp/stdio; W1/W2 | Implementers who choose to support custom transports MUST ensure they preserve the |
| `spec/basic/transports.mdx:287` Custom Transports | SHOULD; optional custom transport implementer | inapplicable with reason (custom); mcp/stdio; W1/W2 | SHOULD document their specific connection establishment and message exchange patterns |
| `spec/server/tools.mdx:22` User Interaction Model | SHOULD; user application | client-owned SHOULD; server UI inapplicable (ui); mcp/stdio; W7 | For trust & safety and security, there SHOULD always |
| `spec/server/tools.mdx:25` User Interaction Model | SHOULD; user application | client-owned SHOULD; server UI inapplicable (ui); mcp/stdio; W7 | Applications SHOULD: |
| `spec/server/tools.mdx:36` Capabilities | MUST; tool server | conforms (capability); mcp; W1 | Servers that support tools MUST declare the `tools` capability: |
| `spec/server/tools.mdx:140` List Changed Notification | SHOULD; server declaring listChanged | inapplicable with reason (list-change); mcp; W7 | capability SHOULD send a notification: |
| `spec/server/tools.mdx:188` Tool | MUST; tool client | opaque bridge conforms; model trust client-owned (annotations); mcp/stdio; W7 | For trust & safety and security, clients MUST consider |
| `spec/server/tools.mdx:228` Embedded Resources | MAY; server choosing resource content | inapplicable with reason (resource); mcp; W7 | Resources MAY be embedded, to provide additional context |
| `spec/server/tools.mdx:289` Security Considerations | MUST; tool server | conforms; rate repaired; four child rows below (server-tools); mcp/api; W6/W7/W9 | 1. Servers MUST: |
| `spec/server/tools.mdx:295` Security Considerations | SHOULD; model/UI client | application-owned; bridge timeout conforms (client-tools); stdio; W7/W8 | 2. Clients SHOULD: |

### 2025-06-18

| Source / heading | Level / actor | Disposition; owner; witness | Literal requirement cue |
| --- | --- | --- | --- |
| `spec/basic/index.mdx:17` Overview | MUST; both | retained deviation C1/C2/C3/C4/C6 (base); mcp/stdio; W1/W3/W4 | All implementations MUST support the base protocol and lifecycle management |
| `spec/basic/index.mdx:18` Overview | MAY; both | conforms negotiated operations; C4 invalid forwarding (operation); mcp/stdio; W1/W3/W5/W7 | components. Other components MAY be implemented based on the specific needs of the |
| `spec/basic/index.mdx:27` Messages | MUST; senders | retained deviation C2/C4/C6 (format); mcp/stdio; W1/W3/W4 | All messages between MCP clients and servers MUST follow the |
| `spec/basic/index.mdx:46` Requests | MUST; request/response sender | retained deviation C2; server receiver tolerance retained (ids); mcp/stdio; W1/W4 | - Requests MUST include a string or integer ID. |
| `spec/basic/index.mdx:47` Requests | MUST NOT; request/response sender | retained deviation C2; server receiver tolerance retained (ids); mcp/stdio; W1/W4 | - Unlike base JSON-RPC, the ID MUST NOT be `null`. |
| `spec/basic/index.mdx:48` Requests | MUST NOT; request/response sender | retained deviation C2; server receiver tolerance retained (ids); mcp/stdio; W1/W4 | - The request ID MUST NOT have been previously used by the requestor within the same |
| `spec/basic/index.mdx:70` Responses | MUST; request/response sender | retained deviation C2; server receiver tolerance retained (ids); mcp/stdio; W1/W4 | - Responses MUST include the same ID as the request they correspond to. |
| `spec/basic/index.mdx:72` Responses | MUST/MUST NOT; response sender | conforms for owned valid-ID responses; C6 for forwarded replies (response); mcp/stdio; W1/W4 | errors. Either a `result` or an `error` MUST be set. A response MUST NOT |
| `spec/basic/index.mdx:74` Responses | MAY/MUST; response sender | conforms for owned valid-ID responses; C6 for forwarded replies (response); mcp/stdio; W1/W4 | - Results MAY follow any JSON object structure, while errors MUST include an |
| `spec/basic/index.mdx:76` Responses | MUST; response sender | conforms for owned valid-ID responses; C6 for forwarded replies (response); mcp/stdio; W1/W4 | - Error codes MUST be integers. |
| `spec/basic/index.mdx:81` Notifications | MUST NOT; sender/receiver | conforms for valid notifications; C4 for invalid-input forwarding (notification); mcp/stdio; W1/W3 | The receiver MUST NOT send a response. |
| `spec/basic/index.mdx:93` Notifications | MUST NOT; sender/receiver | conforms for valid notifications; C4 for invalid-input forwarding (notification); mcp/stdio; W1/W3 | - Notifications MUST NOT include an ID. |
| `spec/basic/index.mdx:98` Auth | SHOULD; HTTP implementation; stdio client | justified SHOULD departure (auth); api/config; W6 | Implementations using an HTTP-based transport SHOULD conform to this specification, |
| `spec/basic/index.mdx:99` Auth | SHOULD NOT; HTTP implementation; stdio client | justified SHOULD departure (auth); api/config; W6 | whereas implementations using STDIO transport SHOULD NOT follow this specification, |
| `spec/basic/index.mdx:102` Auth | MAY; HTTP implementation; stdio client | justified SHOULD departure (auth); api/config; W6 | Additionally, clients and servers MAY negotiate their own custom authentication and |
| `spec/basic/index.mdx:129` `_meta` | MUST NOT; metadata producer/consumer | conforms consumption; production inapplicable (meta); mcp/stdio; W7 | implementations MUST NOT make assumptions about values at these keys. |
| `spec/basic/index.mdx:138` `_meta` | MUST; metadata producer/consumer | conforms consumption; production inapplicable (meta); mcp/stdio; W7 | - If specified, MUST be a series of labels separated by dots (`.`), followed by a slash (`/`). |
| `spec/basic/index.mdx:139` `_meta` | MUST; metadata producer/consumer | conforms consumption; production inapplicable (meta); mcp/stdio; W7 | - Labels MUST start with a letter and end with a letter or digit; interior characters can be letters, digits, or hyphens (`-`). |
| `spec/basic/index.mdx:146` `_meta` | MUST; metadata producer/consumer | conforms consumption; production inapplicable (meta); mcp/stdio; W7 | - Unless empty, MUST begin and end with an alphanumeric character (`[a-z0-9A-Z]`). |
| `spec/basic/index.mdx:147` `_meta` | MAY; metadata producer/consumer | conforms consumption; production inapplicable (meta); mcp/stdio; W7 | - MAY contain hyphens (`-`), underscores (`_`), dots (`.`), and alphanumerics in between. |
| `spec/basic/lifecycle.mdx:40` Initialization | MUST; initialization client | C4 for compatibility; conforms valid modern flow (init-client); stdio/mcp; W1/W3 | The initialization phase MUST be the first interaction between client and server. |
| `spec/basic/lifecycle.mdx:47` Initialization | MUST; initialization client | C4 for compatibility; conforms valid modern flow (init-client); stdio/mcp; W1/W3 | The client MUST initiate this phase by sending an `initialize` request containing: |
| `spec/basic/lifecycle.mdx:76` Initialization | MUST; server; bridge state owner | conforms; required-result installation repaired (init-server); mcp/stdio; W1/W3 | The server MUST respond with its own capabilities and information: |
| `spec/basic/lifecycle.mdx:107` Initialization | MUST; initialization client | C4 for compatibility; conforms valid modern flow (init-client); stdio/mcp; W1/W3 | After successful initialization, the client MUST send an `initialized` notification |
| `spec/basic/lifecycle.mdx:117` Initialization | SHOULD NOT; client | modern fence conforms; legacy SHOULD departure (pre-client); stdio; W3 | - The client SHOULD NOT send requests other than |
| `spec/basic/lifecycle.mdx:120` Initialization | SHOULD NOT; server | conforms (pre-server); mcp; W3 | - The server SHOULD NOT send requests other than |
| `spec/basic/lifecycle.mdx:127` Version Negotiation | MUST; initialization sender/server | conforms valid single initialize; C4 invalid fixtures (version); mcp/stdio; W1/W3 | In the `initialize` request, the client MUST send a protocol version it supports. |
| `spec/basic/lifecycle.mdx:128` Version Negotiation | SHOULD; initialization sender/server | conforms valid single initialize; C4 invalid fixtures (version); mcp/stdio; W1/W3 | This SHOULD be the _latest_ version supported by the client. |
| `spec/basic/lifecycle.mdx:130` Version Negotiation | MUST; initialization sender/server | conforms valid single initialize; C4 invalid fixtures (version); mcp/stdio; W1/W3 | If the server supports the requested protocol version, it MUST respond with the same |
| `spec/basic/lifecycle.mdx:131` Version Negotiation | MUST; initialization sender/server | conforms valid single initialize; C4 invalid fixtures (version); mcp/stdio; W1/W3 | version. Otherwise, the server MUST respond with another protocol version it |
| `spec/basic/lifecycle.mdx:132` Version Negotiation | SHOULD; initialization sender/server | conforms valid single initialize; C4 invalid fixtures (version); mcp/stdio; W1/W3 | supports. This SHOULD be the _latest_ version supported by the server. |
| `spec/basic/lifecycle.mdx:134` Version Negotiation | SHOULD; initialization sender/server | conforms valid single initialize; C4 invalid fixtures (version); mcp/stdio; W1/W3 | If the client does not support the version in the server's response, it SHOULD |
| `spec/basic/lifecycle.mdx:138` Version Negotiation | MUST; HTTP client/server | conforms; non-POST refusal repaired (header); mcp/api/stdio; W1/W2/W3 | If using HTTP, the client MUST include the `MCP-Protocol-Version: |
| `spec/basic/lifecycle.mdx:175` Operation | MUST; both | conforms negotiated operations; C4 invalid forwarding (operation); mcp/stdio; W1/W3/W5/W7 | Both parties MUST: |
| `spec/basic/lifecycle.mdx:188` stdio | SHOULD; stdio launcher; optional server | bridge conforms; launcher inapplicable (shutdown); stdio; W8 | For the stdio transport, the client SHOULD initiate |
| `spec/basic/lifecycle.mdx:196` stdio | MAY; stdio launcher; optional server | bridge conforms; launcher inapplicable (shutdown); stdio; W8 | The server MAY initiate shutdown by closing its output stream to the client and |
| `spec/basic/lifecycle.mdx:206` Timeouts | SHOULD; sender/middleware | conforms with SHOULD departures (timeout); stdio; W8 | Implementations SHOULD establish timeouts for all sent requests, to prevent hung |
| `spec/basic/lifecycle.mdx:208` Timeouts | SHOULD; sender/middleware | conforms with SHOULD departures (timeout); stdio; W8 | response within the timeout period, the sender SHOULD issue a cancellation |
| `spec/basic/lifecycle.mdx:212` Timeouts | SHOULD; sender/middleware | conforms with SHOULD departures (timeout); stdio; W8 | SDKs and other middleware SHOULD allow these timeouts to be configured on a |
| `spec/basic/lifecycle.mdx:215` Timeouts | MAY; sender/middleware | conforms with SHOULD departures (timeout); stdio; W8 | Implementations MAY choose to reset the timeout clock when receiving a [progress |
| `spec/basic/lifecycle.mdx:217` Timeouts | SHOULD; sender/middleware | conforms with SHOULD departures (timeout); stdio; W8 | implies that work is actually happening. However, implementations SHOULD always |
| `spec/basic/lifecycle.mdx:223` Error Handling | SHOULD; implementation | conforms (errors); mcp/stdio; W1/W3/W8 | Implementations SHOULD be prepared to handle these error cases: |
| `spec/basic/transports.mdx:7` Overview | MUST; transport senders | conforms owned bytes; semantic C6 separate (utf8); mcp/stdio; W8 | MCP uses JSON-RPC to encode messages. JSON-RPC messages MUST be UTF-8 encoded. |
| `spec/basic/transports.mdx:15` Overview | SHOULD; client | conforms (stdio-support); stdio; W1/W8 | Clients SHOULD support stdio whenever possible. |
| `spec/basic/transports.mdx:28` stdio | MUST NOT; stdio senders | conforms (framing); stdio; W8 | - Messages are delimited by newlines, and MUST NOT contain embedded newlines. |
| `spec/basic/transports.mdx:29` stdio | MAY; server; stderr consumer | conforms (logging); stdio/main; W8 | - The server MAY write UTF-8 strings to its standard error (`stderr`) for logging |
| `spec/basic/transports.mdx:30` stdio | MAY; server; stderr consumer | conforms (logging); stdio/main; W8 | purposes. Clients MAY capture, forward, or ignore this logging. |
| `spec/basic/transports.mdx:31` stdio | MUST NOT; stdio server | retained deviation C2/C6; owned framing conforms (stdout); stdio; W4/W8 | - The server MUST NOT write anything to its `stdout` that is not a valid MCP message. |
| `spec/basic/transports.mdx:32` stdio | MUST NOT; external stdio client | producer inapplicable; bridge forwarding C4 (stdin); stdio; W1/W8 | - The client MUST NOT write anything to the server's `stdin` that is not a valid MCP |
| `spec/basic/transports.mdx:68` Streamable HTTP | MUST; HTTP server | conforms (endpoint); api; W2 | The server MUST provide a single HTTP endpoint path (hereafter referred to as the |
| `spec/basic/transports.mdx:76` Security Warning | MUST; HTTP server | conforms (origin); api; W2/W6 | 1. Servers MUST validate the `Origin` header on all incoming connections to prevent DNS rebinding attacks |
| `spec/basic/transports.mdx:77` Security Warning | SHOULD; HTTP implementation; stdio client | justified SHOULD departure (auth); api/config; W6 | 2. When running locally, servers SHOULD bind only to localhost (127.0.0.1) rather than all network interfaces (0.0.0.0) |
| `spec/basic/transports.mdx:78` Security Warning | SHOULD; HTTP implementation; stdio client | justified SHOULD departure (auth); api/config; W6 | 3. Servers SHOULD implement proper authentication for all connections |
| `spec/basic/transports.mdx:84` Sending Messages to the Server | MUST; HTTP client | conforms valid input; C4 invalid forwarding (post); stdio; W3 | Every JSON-RPC message sent from the client MUST be a new HTTP POST request to the |
| `spec/basic/transports.mdx:87` Sending Messages to the Server | MUST; HTTP client | conforms valid input; C4 invalid forwarding (post); stdio; W3 | 1. The client MUST use HTTP POST to send JSON-RPC messages to the MCP endpoint. |
| `spec/basic/transports.mdx:88` Sending Messages to the Server | MUST; HTTP client | conforms valid input; C4 invalid forwarding (post); stdio; W3 | 2. The client MUST include an `Accept` header, listing both `application/json` and |
| `spec/basic/transports.mdx:90` Sending Messages to the Server | MUST; HTTP client | conforms valid input; C4 invalid forwarding (post); stdio; W3 | 3. The body of the POST request MUST be a single JSON-RPC _request_, _notification_, or _response_. |
| `spec/basic/transports.mdx:92` Sending Messages to the Server | MUST; HTTP server | notifications conform; responses retained deviation C1 (accepted-input); mcp; W4 | - If the server accepts the input, the server MUST return HTTP status code 202 |
| `spec/basic/transports.mdx:94` Sending Messages to the Server | MUST; HTTP server | notifications conform; responses retained deviation C1 (accepted-input); mcp; W4 | - If the server cannot accept the input, it MUST return an HTTP error status code |
| `spec/basic/transports.mdx:95` Sending Messages to the Server | MAY; HTTP server | notifications conform; responses retained deviation C1 (accepted-input); mcp; W4 | (e.g., 400 Bad Request). The HTTP response body MAY comprise a JSON-RPC _error |
| `spec/basic/transports.mdx:97` Sending Messages to the Server | MUST; HTTP server/client | JSON server conforms; client retained deviation C3 (json-or-sse); mcp/stdio; W2/W3 | 5. If the input is a JSON-RPC _request_, the server MUST either |
| `spec/basic/transports.mdx:99` Sending Messages to the Server | MUST; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | `Content-Type: application/json`, to return one JSON object. The client MUST |
| `spec/basic/transports.mdx:102` Sending Messages to the Server | SHOULD; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - The SSE stream SHOULD eventually include JSON-RPC _response_ for the |
| `spec/basic/transports.mdx:104` Sending Messages to the Server | MAY; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - The server MAY send JSON-RPC _requests_ and _notifications_ before sending the |
| `spec/basic/transports.mdx:105` Sending Messages to the Server | SHOULD; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | JSON-RPC _response_. These messages SHOULD relate to the originating client |
| `spec/basic/transports.mdx:107` Sending Messages to the Server | SHOULD NOT; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - The server SHOULD NOT close the SSE stream before sending the JSON-RPC _response_ |
| `spec/basic/transports.mdx:110` Sending Messages to the Server | SHOULD; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - After the JSON-RPC _response_ has been sent, the server SHOULD close the SSE |
| `spec/basic/transports.mdx:112` Sending Messages to the Server | MAY; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - Disconnection MAY occur at any time (e.g., due to network conditions). |
| `spec/basic/transports.mdx:114` Sending Messages to the Server | SHOULD NOT; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - Disconnection SHOULD NOT be interpreted as the client cancelling its request. |
| `spec/basic/transports.mdx:115` Sending Messages to the Server | SHOULD; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - To cancel, the client SHOULD explicitly send an MCP `CancelledNotification`. |
| `spec/basic/transports.mdx:116` Sending Messages to the Server | MAY; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - To avoid message loss due to disconnection, the server MAY make the stream |
| `spec/basic/transports.mdx:121` Listening for Messages from the Server | MAY; optional client; server | server conforms; optional client unused (get); api/stdio; W2 | 1. The client MAY issue an HTTP GET to the MCP endpoint. This can be used to open an |
| `spec/basic/transports.mdx:124` Listening for Messages from the Server | MUST; optional client; server | server conforms; optional client unused (get); api/stdio; W2 | 2. The client MUST include an `Accept` header, listing `text/event-stream` as a |
| `spec/basic/transports.mdx:126` Listening for Messages from the Server | MUST; optional client; server | server conforms; optional client unused (get); api/stdio; W2 | 3. The server MUST either return `Content-Type: text/event-stream` in response to |
| `spec/basic/transports.mdx:130` Listening for Messages from the Server | MAY; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - The server MAY send JSON-RPC _requests_ and _notifications_ on the stream. |
| `spec/basic/transports.mdx:131` Listening for Messages from the Server | SHOULD; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - These messages SHOULD be unrelated to any concurrently-running JSON-RPC |
| `spec/basic/transports.mdx:133` Listening for Messages from the Server | MUST NOT; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - The server MUST NOT send a JSON-RPC _response_ on the stream unless |
| `spec/basic/transports.mdx:136` Listening for Messages from the Server | MAY; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - The server MAY close the SSE stream at any time. |
| `spec/basic/transports.mdx:137` Listening for Messages from the Server | MAY; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - The client MAY close the SSE stream at any time. |
| `spec/basic/transports.mdx:141` Multiple Connections | MAY; SSE sender/receiver | inapplicable with reason (connections); mcp; W2 | 1. The client MAY remain connected to multiple SSE streams simultaneously. |
| `spec/basic/transports.mdx:142` Multiple Connections | MUST; SSE sender/receiver | inapplicable with reason (connections); mcp; W2 | 2. The server MUST send each of its JSON-RPC messages on only one of the connected |
| `spec/basic/transports.mdx:143` Multiple Connections | MUST NOT; SSE sender/receiver | inapplicable with reason (connections); mcp; W2 | streams; that is, it MUST NOT broadcast the same message across multiple streams. |
| `spec/basic/transports.mdx:144` Multiple Connections | MAY; SSE sender/receiver | inapplicable with reason (connections); mcp; W2 | - The risk of message loss MAY be mitigated by making the stream |
| `spec/basic/transports.mdx:152` Resumability and Redelivery | MAY; SSE sender/receiver | server inapplicable; general client C3 (resume); mcp/stdio; W2 | 1. Servers MAY attach an `id` field to their SSE events, as described in the |
| `spec/basic/transports.mdx:154` Resumability and Redelivery | MUST; SSE sender/receiver | server inapplicable; general client C3 (resume); mcp/stdio; W2 | - If present, the ID MUST be globally unique across all streams within that |
| `spec/basic/transports.mdx:157` Resumability and Redelivery | SHOULD; SSE sender/receiver | server inapplicable; general client C3 (resume); mcp/stdio; W2 | 2. If the client wishes to resume after a broken connection, it SHOULD issue an HTTP |
| `spec/basic/transports.mdx:161` Resumability and Redelivery | MAY; SSE sender/receiver | server inapplicable; general client C3 (resume); mcp/stdio; W2 | - The server MAY use this header to replay messages that would have been sent |
| `spec/basic/transports.mdx:164` Resumability and Redelivery | MUST NOT; SSE sender/receiver | server inapplicable; general client C3 (resume); mcp/stdio; W2 | - The server MUST NOT replay messages that would have been delivered on a |
| `spec/basic/transports.mdx:176` Session Management | MAY; optional stateful server; conditional client | paired server inapplicable; external client C3 (session); mcp/stdio; W1/W2/W3 | 1. A server using the Streamable HTTP transport MAY assign a session ID at |
| `spec/basic/transports.mdx:179` Session Management | SHOULD; optional stateful server; conditional client | paired server inapplicable; external client C3 (session); mcp/stdio; W1/W2/W3 | - The session ID SHOULD be globally unique and cryptographically secure (e.g., a |
| `spec/basic/transports.mdx:181` Session Management | MUST; optional stateful server; conditional client | paired server inapplicable; external client C3 (session); mcp/stdio; W1/W2/W3 | - The session ID MUST only contain visible ASCII characters (ranging from 0x21 to |
| `spec/basic/transports.mdx:184` Session Management | MUST; optional stateful server; conditional client | paired server inapplicable; external client C3 (session); mcp/stdio; W1/W2/W3 | the Streamable HTTP transport MUST include it in the `Mcp-Session-Id` header on |
| `spec/basic/transports.mdx:186` Session Management | SHOULD; optional stateful server; conditional client | paired server inapplicable; external client C3 (session); mcp/stdio; W1/W2/W3 | - Servers that require a session ID SHOULD respond to requests without an |
| `spec/basic/transports.mdx:188` Session Management | MAY/MUST; optional stateful server; conditional client | paired server inapplicable; external client C3 (session); mcp/stdio; W1/W2/W3 | 3. The server MAY terminate the session at any time, after which it MUST respond |
| `spec/basic/transports.mdx:191` Session Management | MUST; optional stateful server; conditional client | paired server inapplicable; external client C3 (session); mcp/stdio; W1/W2/W3 | `Mcp-Session-Id`, it MUST start a new session by sending a new `InitializeRequest` |
| `spec/basic/transports.mdx:194` Session Management | SHOULD; optional stateful server; conditional client | paired server inapplicable; external client C3 (session); mcp/stdio; W1/W2/W3 | the client application) SHOULD send an HTTP DELETE to the MCP endpoint with the |
| `spec/basic/transports.mdx:196` Session Management | MAY; optional stateful server; conditional client | paired server inapplicable; external client C3 (session); mcp/stdio; W1/W2/W3 | - The server MAY respond to this request with HTTP 405 Method Not Allowed, |
| `spec/basic/transports.mdx:242` Protocol Version Header | MUST; HTTP client/server | conforms; non-POST refusal repaired (header); mcp/api/stdio; W1/W2/W3 | If using HTTP, the client MUST include the `MCP-Protocol-Version: |
| `spec/basic/transports.mdx:248` Protocol Version Header | SHOULD; HTTP client/server | conforms; non-POST refusal repaired (header); mcp/api/stdio; W1/W2/W3 | The protocol version sent by the client SHOULD be the one [negotiated during |
| `spec/basic/transports.mdx:253` Protocol Version Header | SHOULD; HTTP client/server | conforms; non-POST refusal repaired (header); mcp/api/stdio; W1/W2/W3 | protocol version negotiated during initialization - the server SHOULD assume protocol |
| `spec/basic/transports.mdx:257` Protocol Version Header | MUST; HTTP client/server | conforms; non-POST refusal repaired (header); mcp/api/stdio; W1/W2/W3 | `MCP-Protocol-Version`, it MUST respond with `400 Bad Request`. |
| `spec/basic/transports.mdx:290` Custom Transports | MAY; optional custom transport implementer | inapplicable with reason (custom); mcp/stdio; W1/W2 | Clients and servers MAY implement additional custom transport mechanisms to suit |
| `spec/basic/transports.mdx:294` Custom Transports | MUST; optional custom transport implementer | inapplicable with reason (custom); mcp/stdio; W1/W2 | Implementers who choose to support custom transports MUST ensure they preserve the |
| `spec/basic/transports.mdx:296` Custom Transports | SHOULD; optional custom transport implementer | inapplicable with reason (custom); mcp/stdio; W1/W2 | SHOULD document their specific connection establishment and message exchange patterns |
| `spec/server/tools.mdx:24` User Interaction Model | SHOULD; user application | client-owned SHOULD; server UI inapplicable (ui); mcp/stdio; W7 | For trust & safety and security, there SHOULD always |
| `spec/server/tools.mdx:27` User Interaction Model | SHOULD; user application | client-owned SHOULD; server UI inapplicable (ui); mcp/stdio; W7 | Applications SHOULD: |
| `spec/server/tools.mdx:38` Capabilities | MUST; tool server | conforms (capability); mcp; W1 | Servers that support tools MUST declare the `tools` capability: |
| `spec/server/tools.mdx:143` List Changed Notification | SHOULD; server declaring listChanged | inapplicable with reason (list-change); mcp; W7 | capability SHOULD send a notification: |
| `spec/server/tools.mdx:193` Tool | MUST; tool client | opaque bridge conforms; model trust client-owned (annotations); mcp/stdio; W7 | For trust & safety and security, clients MUST consider |
| `spec/server/tools.mdx:249` Resource Links | MAY; server choosing resource content | inapplicable with reason (resource); mcp; W7 | A tool MAY return links to [Resources, to provide additional context |
| `spec/server/tools.mdx:275` Embedded Resources | MAY; server choosing resource content | inapplicable with reason (resource); mcp; W7 | Resources MAY be embedded to provide additional context |
| `spec/server/tools.mdx:276` Embedded Resources | SHOULD; server choosing resource content | inapplicable with reason (resource); mcp; W7 | or data using a suitable URI scheme. Servers that use embedded resources SHOULD implement the `resources` capability: |
| `spec/server/tools.mdx:300` Structured Content | SHOULD; server choosing structured output/schema | inapplicable with reason (structured); mcp; W7 | For backwards compatibility, a tool that returns structured content SHOULD also return the serialized JSON in a TextContent block. |
| `spec/server/tools.mdx:314` Output Schema | MUST; server choosing structured output/schema | inapplicable with reason (structured); mcp; W7 | - Servers MUST provide structured results that conform to this schema. |
| `spec/server/tools.mdx:315` Output Schema | SHOULD; server choosing structured output/schema | inapplicable with reason (structured); mcp; W7 | - Clients SHOULD validate structured results against this schema. |
| `spec/server/tools.mdx:431` Security Considerations | MUST; tool server | conforms; rate repaired; four child rows below (server-tools); mcp/api; W6/W7/W9 | 1. Servers MUST: |
| `spec/server/tools.mdx:437` Security Considerations | SHOULD; model/UI client | application-owned; bridge timeout conforms (client-tools); stdio; W7/W8 | 2. Clients SHOULD: |

### 2025-11-25

| Source / heading | Level / actor | Disposition; owner; witness | Literal requirement cue |
| --- | --- | --- | --- |
| `spec/basic/index.mdx:17` Overview | MUST; both | retained deviation C1/C2/C3/C4/C6 (base); mcp/stdio; W1/W3/W4 | All implementations MUST support the base protocol and lifecycle management |
| `spec/basic/index.mdx:18` Overview | MAY; both | conforms negotiated operations; C4 invalid forwarding (operation); mcp/stdio; W1/W3/W5/W7 | components. Other components MAY be implemented based on the specific needs of the |
| `spec/basic/index.mdx:27` Messages | MUST; senders | retained deviation C2/C4/C6 (format); mcp/stdio; W1/W3/W4 | All messages between MCP clients and servers MUST follow the |
| `spec/basic/index.mdx:46` Requests | MUST; request/response sender | retained deviation C2; server receiver tolerance retained (ids); mcp/stdio; W1/W4 | - Requests MUST include a string or integer ID. |
| `spec/basic/index.mdx:47` Requests | MUST NOT; request/response sender | retained deviation C2; server receiver tolerance retained (ids); mcp/stdio; W1/W4 | - Unlike base JSON-RPC, the ID MUST NOT be `null`. |
| `spec/basic/index.mdx:48` Requests | MUST NOT; request/response sender | retained deviation C2; server receiver tolerance retained (ids); mcp/stdio; W1/W4 | - The request ID MUST NOT have been previously used by the requestor within the same |
| `spec/basic/index.mdx:69` Result Responses | MUST; request/response sender | retained deviation C2; server receiver tolerance retained (ids); mcp/stdio; W1/W4 | - Result responses MUST include the same ID as the request they correspond to. |
| `spec/basic/index.mdx:70` Result Responses | MUST; response sender | conforms for owned valid-ID responses; C6 for forwarded replies (response); mcp/stdio; W1/W4 | - Result responses MUST include a `result` field. |
| `spec/basic/index.mdx:71` Result Responses | MAY; response sender | conforms for owned valid-ID responses; C6 for forwarded replies (response); mcp/stdio; W1/W4 | - The `result` MAY follow any JSON object structure. |
| `spec/basic/index.mdx:89` Error Responses | MUST; request/response sender | retained deviation C2; server receiver tolerance retained (ids); mcp/stdio; W1/W4 | - Error responses MUST include the same ID as the request they correspond to (except in error cases where the ID could not be read due a malformed request). |
| `spec/basic/index.mdx:90` Error Responses | MUST; response sender | conforms for owned valid-ID responses; C6 for forwarded replies (response); mcp/stdio; W1/W4 | - Error responses MUST include an `error` field with a `code` and `message`. |
| `spec/basic/index.mdx:91` Error Responses | MUST; response sender | conforms for owned valid-ID responses; C6 for forwarded replies (response); mcp/stdio; W1/W4 | - Error codes MUST be integers. |
| `spec/basic/index.mdx:96` Notifications | MUST NOT; sender/receiver | conforms for valid notifications; C4 for invalid-input forwarding (notification); mcp/stdio; W1/W3 | The receiver MUST NOT send a response. |
| `spec/basic/index.mdx:108` Notifications | MUST NOT; sender/receiver | conforms for valid notifications; C4 for invalid-input forwarding (notification); mcp/stdio; W1/W3 | - Notifications MUST NOT include an ID. |
| `spec/basic/index.mdx:113` Auth | SHOULD; HTTP implementation; stdio client | justified SHOULD departure (auth); api/config; W6 | Implementations using an HTTP-based transport SHOULD conform to this specification, |
| `spec/basic/index.mdx:114` Auth | SHOULD NOT; HTTP implementation; stdio client | justified SHOULD departure (auth); api/config; W6 | whereas implementations using STDIO transport SHOULD NOT follow this specification, |
| `spec/basic/index.mdx:117` Auth | MAY; HTTP implementation; stdio client | justified SHOULD departure (auth); api/config; W6 | Additionally, clients and servers MAY negotiate their own custom authentication and |
| `spec/basic/index.mdx:145` Schema Dialect | MAY; schema producer/consumer | conforms for owned schemas; external consumption inapplicable (dialect); mcp; W7 | 1. Explicit dialect: Schemas MAY include a `$schema` field to specify a different dialect |
| `spec/basic/index.mdx:146` Schema Dialect | MUST/SHOULD; schema producer/consumer | conforms for owned schemas; external consumption inapplicable (dialect); mcp; W7 | 1. Supported dialects: Implementations MUST support at least 2020-12 and SHOULD document which additional dialects they support |
| `spec/basic/index.mdx:147` Schema Dialect | RECOMMENDED; schema producer/consumer | conforms for owned schemas; external consumption inapplicable (dialect); mcp; W7 | 1. Recommendation: Implementors are RECOMMENDED to use JSON Schema 2020-12. |
| `spec/basic/index.mdx:180` Implementation Requirements | MUST; schema producer/consumer | conforms for owned schemas; external consumption inapplicable (dialect); mcp; W7 | - Clients and servers MUST support JSON Schema 2020-12 for schemas without an explicit `$schema` field |
| `spec/basic/index.mdx:181` Implementation Requirements | MUST; schema producer/consumer | conforms for owned schemas; external consumption inapplicable (dialect); mcp; W7 | - Clients and servers MUST validate schemas according to their declared or default dialect. They MUST handle unsupported dialects gracefully by returning an appropriate error indicating the dialect is not supported. |
| `spec/basic/index.mdx:182` Implementation Requirements | SHOULD; schema producer/consumer | conforms for owned schemas; external consumption inapplicable (dialect); mcp; W7 | - Clients and servers SHOULD document which schema dialects they support |
| `spec/basic/index.mdx:186` Schema Validation | MUST; schema producer/consumer | conforms for owned schemas; external consumption inapplicable (dialect); mcp; W7 | - Schemas MUST be valid according to their declared or default dialect |
| `spec/basic/index.mdx:196` `_meta` | MUST NOT; metadata producer/consumer | conforms consumption; production inapplicable (meta); mcp/stdio; W7 | implementations MUST NOT make assumptions about values at these keys. |
| `spec/basic/index.mdx:205` `_meta` | MUST; metadata producer/consumer | conforms consumption; production inapplicable (meta); mcp/stdio; W7 | - If specified, MUST be a series of labels separated by dots (`.`), followed by a slash (`/`). |
| `spec/basic/index.mdx:206` `_meta` | MUST; metadata producer/consumer | conforms consumption; production inapplicable (meta); mcp/stdio; W7 | - Labels MUST start with a letter and end with a letter or digit; interior characters can be letters, digits, or hyphens (`-`). |
| `spec/basic/index.mdx:207` `_meta` | SHOULD; metadata producer/consumer | conforms consumption; production inapplicable (meta); mcp/stdio; W7 | - Implementations SHOULD use reverse DNS notation (e.g., `com.example/` rather than `example.com/`). |
| `spec/basic/index.mdx:214` `_meta` | MUST; metadata producer/consumer | conforms consumption; production inapplicable (meta); mcp/stdio; W7 | - Unless empty, MUST begin and end with an alphanumeric character (`[a-z0-9A-Z]`). |
| `spec/basic/index.mdx:215` `_meta` | MAY; metadata producer/consumer | conforms consumption; production inapplicable (meta); mcp/stdio; W7 | - MAY contain hyphens (`-`), underscores (`_`), dots (`.`), and alphanumerics in between. |
| `spec/basic/index.mdx:232` `icons` | MUST; optional producer; renderer | inapplicable with reason (icons); mcp/stdio; W7 | Clients that support rendering icons MUST support at least the following MIME types: |
| `spec/basic/index.mdx:237` `icons` | SHOULD; optional producer; renderer | inapplicable with reason (icons); mcp/stdio; W7 | Clients that support rendering icons SHOULD also support: |
| `spec/basic/index.mdx:244` `icons` | MUST; optional producer; renderer | inapplicable with reason (icons); mcp/stdio; W7 | Consumers of icon metadata MUST take appropriate security precautions when handling icons to prevent compromise: |
| `spec/basic/index.mdx:247` `icons` | MUST; optional producer; renderer | inapplicable with reason (icons); mcp/stdio; W7 | - Ensure that the icon URI is either a HTTPS or `data:` URI. Clients MUST reject icon URIs that use unsafe schemes and redirects, such as `javascript:`, `file:`, `ftp:`, `ws:`, or local app URI schemes. |
| `spec/basic/index.mdx:250` `icons` | MAY; optional producer; renderer | inapplicable with reason (icons); mcp/stdio; W7 | - Consumers MAY set limits for image and content size. |
| `spec/basic/index.mdx:253` `icons` | MAY; optional producer; renderer | inapplicable with reason (icons); mcp/stdio; W7 | - Exercise caution when fetching and rendering icons as the payload MAY contain executable content (e.g., SVG with [embedded JavaScript](https://www.w3.org/TR/SVG11/script.html) or [extended capabilities](https://www.w3.org/TR/SVG11/extend.html)). |
| `spec/basic/index.mdx:254` `icons` | MAY; optional producer; renderer | inapplicable with reason (icons); mcp/stdio; W7 | - Consumers MAY choose to disallow specific file types or otherwise sanitize icon files before rendering. |
| `spec/basic/lifecycle.mdx:40` Initialization | MUST; initialization client | C4 for compatibility; conforms valid modern flow (init-client); stdio/mcp; W1/W3 | The initialization phase MUST be the first interaction between client and server. |
| `spec/basic/lifecycle.mdx:47` Initialization | MUST; initialization client | C4 for compatibility; conforms valid modern flow (init-client); stdio/mcp; W1/W3 | The client MUST initiate this phase by sending an `initialize` request containing: |
| `spec/basic/lifecycle.mdx:98` Initialization | MUST; server; bridge state owner | conforms; required-result installation repaired (init-server); mcp/stdio; W1/W3 | The server MUST respond with its own capabilities and information: |
| `spec/basic/lifecycle.mdx:147` Initialization | MUST; initialization client | C4 for compatibility; conforms valid modern flow (init-client); stdio/mcp; W1/W3 | After successful initialization, the client MUST send an `initialized` notification |
| `spec/basic/lifecycle.mdx:157` Initialization | SHOULD NOT; client | modern fence conforms; legacy SHOULD departure (pre-client); stdio; W3 | - The client SHOULD NOT send requests other than |
| `spec/basic/lifecycle.mdx:160` Initialization | SHOULD NOT; server | conforms (pre-server); mcp; W3 | - The server SHOULD NOT send requests other than |
| `spec/basic/lifecycle.mdx:167` Version Negotiation | MUST; initialization sender/server | conforms valid single initialize; C4 invalid fixtures (version); mcp/stdio; W1/W3 | In the `initialize` request, the client MUST send a protocol version it supports. |
| `spec/basic/lifecycle.mdx:168` Version Negotiation | SHOULD; initialization sender/server | conforms valid single initialize; C4 invalid fixtures (version); mcp/stdio; W1/W3 | This SHOULD be the _latest_ version supported by the client. |
| `spec/basic/lifecycle.mdx:170` Version Negotiation | MUST; initialization sender/server | conforms valid single initialize; C4 invalid fixtures (version); mcp/stdio; W1/W3 | If the server supports the requested protocol version, it MUST respond with the same |
| `spec/basic/lifecycle.mdx:171` Version Negotiation | MUST; initialization sender/server | conforms valid single initialize; C4 invalid fixtures (version); mcp/stdio; W1/W3 | version. Otherwise, the server MUST respond with another protocol version it |
| `spec/basic/lifecycle.mdx:172` Version Negotiation | SHOULD; initialization sender/server | conforms valid single initialize; C4 invalid fixtures (version); mcp/stdio; W1/W3 | supports. This SHOULD be the _latest_ version supported by the server. |
| `spec/basic/lifecycle.mdx:174` Version Negotiation | SHOULD; initialization sender/server | conforms valid single initialize; C4 invalid fixtures (version); mcp/stdio; W1/W3 | If the client does not support the version in the server's response, it SHOULD |
| `spec/basic/lifecycle.mdx:178` Version Negotiation | MUST; HTTP client/server | conforms; non-POST refusal repaired (header); mcp/api/stdio; W1/W2/W3 | If using HTTP, the client MUST include the `MCP-Protocol-Version: |
| `spec/basic/lifecycle.mdx:217` Operation | MUST; both | conforms negotiated operations; C4 invalid forwarding (operation); mcp/stdio; W1/W3/W5/W7 | Both parties MUST: |
| `spec/basic/lifecycle.mdx:230` stdio | SHOULD; stdio launcher; optional server | bridge conforms; launcher inapplicable (shutdown); stdio; W8 | For the stdio transport, the client SHOULD initiate |
| `spec/basic/lifecycle.mdx:238` stdio | MAY; stdio launcher; optional server | bridge conforms; launcher inapplicable (shutdown); stdio; W8 | The server MAY initiate shutdown by closing its output stream to the client and |
| `spec/basic/lifecycle.mdx:248` Timeouts | SHOULD; sender/middleware | conforms with SHOULD departures (timeout); stdio; W8 | Implementations SHOULD establish timeouts for all sent requests, to prevent hung |
| `spec/basic/lifecycle.mdx:250` Timeouts | SHOULD; sender/middleware | conforms with SHOULD departures (timeout); stdio; W8 | response within the timeout period, the sender SHOULD issue a [cancellation |
| `spec/basic/lifecycle.mdx:254` Timeouts | SHOULD; sender/middleware | conforms with SHOULD departures (timeout); stdio; W8 | SDKs and other middleware SHOULD allow these timeouts to be configured on a |
| `spec/basic/lifecycle.mdx:257` Timeouts | MAY; sender/middleware | conforms with SHOULD departures (timeout); stdio; W8 | Implementations MAY choose to reset the timeout clock when receiving a [progress |
| `spec/basic/lifecycle.mdx:259` Timeouts | SHOULD; sender/middleware | conforms with SHOULD departures (timeout); stdio; W8 | implies that work is actually happening. However, implementations SHOULD always |
| `spec/basic/lifecycle.mdx:265` Error Handling | SHOULD; implementation | conforms (errors); mcp/stdio; W1/W3/W8 | Implementations SHOULD be prepared to handle these error cases: |
| `spec/basic/transports.mdx:7` Overview | MUST; transport senders | conforms owned bytes; semantic C6 separate (utf8); mcp/stdio; W8 | MCP uses JSON-RPC to encode messages. JSON-RPC messages MUST be UTF-8 encoded. |
| `spec/basic/transports.mdx:15` Overview | SHOULD; client | conforms (stdio-support); stdio; W1/W8 | Clients SHOULD support stdio whenever possible. |
| `spec/basic/transports.mdx:28` stdio | MUST NOT; stdio senders | conforms (framing); stdio; W8 | - Messages are delimited by newlines, and MUST NOT contain embedded newlines. |
| `spec/basic/transports.mdx:29` stdio | MAY; server; stderr consumer | conforms (logging); stdio/main; W8 | - The server MAY write UTF-8 strings to its standard error (`stderr`) for any |
| `spec/basic/transports.mdx:31` stdio | MAY; server; stderr consumer | conforms (logging); stdio/main; W8 | - The client MAY capture, forward, or ignore the server's `stderr` output |
| `spec/basic/transports.mdx:32` stdio | SHOULD NOT; server; stderr consumer | conforms (logging); stdio/main; W8 | and SHOULD NOT assume `stderr` output indicates error conditions. |
| `spec/basic/transports.mdx:33` stdio | MUST NOT; stdio server | retained deviation C2/C6; owned framing conforms (stdout); stdio; W4/W8 | - The server MUST NOT write anything to its `stdout` that is not a valid MCP message. |
| `spec/basic/transports.mdx:34` stdio | MUST NOT; external stdio client | producer inapplicable; bridge forwarding C4 (stdin); stdio; W1/W8 | - The client MUST NOT write anything to the server's `stdin` that is not a valid MCP |
| `spec/basic/transports.mdx:70` Streamable HTTP | MUST; HTTP server | conforms (endpoint); api; W2 | The server MUST provide a single HTTP endpoint path (hereafter referred to as the |
| `spec/basic/transports.mdx:78` Security Warning | MUST; HTTP server | conforms (origin); api; W2/W6 | 1. Servers MUST validate the `Origin` header on all incoming connections to prevent DNS rebinding attacks |
| `spec/basic/transports.mdx:79` Security Warning | MUST; HTTP server | conforms (origin); api; W2/W6 | - If the `Origin` header is present and invalid, servers MUST respond with HTTP 403 Forbidden. The HTTP response |
| `spec/basic/transports.mdx:80` Security Warning | MAY; HTTP server | conforms (origin); api; W2/W6 | body MAY comprise a JSON-RPC _error response_ that has no `id` |
| `spec/basic/transports.mdx:81` Security Warning | SHOULD; HTTP implementation; stdio client | justified SHOULD departure (auth); api/config; W6 | 2. When running locally, servers SHOULD bind only to localhost (127.0.0.1) rather than all network interfaces (0.0.0.0) |
| `spec/basic/transports.mdx:82` Security Warning | SHOULD; HTTP implementation; stdio client | justified SHOULD departure (auth); api/config; W6 | 3. Servers SHOULD implement proper authentication for all connections |
| `spec/basic/transports.mdx:88` Sending Messages to the Server | MUST; HTTP client | conforms valid input; C4 invalid forwarding (post); stdio; W3 | Every JSON-RPC message sent from the client MUST be a new HTTP POST request to the |
| `spec/basic/transports.mdx:91` Sending Messages to the Server | MUST; HTTP client | conforms valid input; C4 invalid forwarding (post); stdio; W3 | 1. The client MUST use HTTP POST to send JSON-RPC messages to the MCP endpoint. |
| `spec/basic/transports.mdx:92` Sending Messages to the Server | MUST; HTTP client | conforms valid input; C4 invalid forwarding (post); stdio; W3 | 2. The client MUST include an `Accept` header, listing both `application/json` and |
| `spec/basic/transports.mdx:94` Sending Messages to the Server | MUST; HTTP client | conforms valid input; C4 invalid forwarding (post); stdio; W3 | 3. The body of the POST request MUST be a single JSON-RPC _request_, _notification_, or _response_. |
| `spec/basic/transports.mdx:96` Sending Messages to the Server | MUST; HTTP server | notifications conform; responses retained deviation C1 (accepted-input); mcp; W4 | - If the server accepts the input, the server MUST return HTTP status code 202 |
| `spec/basic/transports.mdx:98` Sending Messages to the Server | MUST; HTTP server | notifications conform; responses retained deviation C1 (accepted-input); mcp; W4 | - If the server cannot accept the input, it MUST return an HTTP error status code |
| `spec/basic/transports.mdx:99` Sending Messages to the Server | MAY; HTTP server | notifications conform; responses retained deviation C1 (accepted-input); mcp; W4 | (e.g., 400 Bad Request). The HTTP response body MAY comprise a JSON-RPC _error |
| `spec/basic/transports.mdx:101` Sending Messages to the Server | MUST; HTTP server/client | JSON server conforms; client retained deviation C3 (json-or-sse); mcp/stdio; W2/W3 | 5. If the input is a JSON-RPC _request_, the server MUST either |
| `spec/basic/transports.mdx:103` Sending Messages to the Server | MUST; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | `Content-Type: application/json`, to return one JSON object. The client MUST |
| `spec/basic/transports.mdx:106` Sending Messages to the Server | SHOULD; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - The server SHOULD immediately send an SSE event consisting of an event |
| `spec/basic/transports.mdx:110` Sending Messages to the Server | MAY; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | server MAY close the _connection_ (without terminating the _SSE stream_) |
| `spec/basic/transports.mdx:112` Sending Messages to the Server | SHOULD; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | SHOULD then "poll" the SSE stream by attempting to reconnect. |
| `spec/basic/transports.mdx:114` Sending Messages to the Server | SHOULD; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | it SHOULD send an SSE event with a standard [`retry`](https://html.spec.whatwg.org/multipage/server-sent-events.html#:~:text=field%20name%20is%20%22retry%22) field before |
| `spec/basic/transports.mdx:115` Sending Messages to the Server | MUST; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | closing the connection. The client MUST respect the `retry` field, |
| `spec/basic/transports.mdx:117` Sending Messages to the Server | SHOULD; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - The SSE stream SHOULD eventually include a JSON-RPC _response_ for the |
| `spec/basic/transports.mdx:119` Sending Messages to the Server | MAY; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - The server MAY send JSON-RPC _requests_ and _notifications_ before sending the |
| `spec/basic/transports.mdx:120` Sending Messages to the Server | SHOULD; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | JSON-RPC _response_. These messages SHOULD relate to the originating client |
| `spec/basic/transports.mdx:122` Sending Messages to the Server | MAY; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - The server MAY terminate the SSE stream if the [session](#session-management) |
| `spec/basic/transports.mdx:124` Sending Messages to the Server | SHOULD; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - After the JSON-RPC _response_ has been sent, the server SHOULD terminate the |
| `spec/basic/transports.mdx:126` Sending Messages to the Server | MAY; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - Disconnection MAY occur at any time (e.g., due to network conditions). |
| `spec/basic/transports.mdx:128` Sending Messages to the Server | SHOULD NOT; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - Disconnection SHOULD NOT be interpreted as the client cancelling its request. |
| `spec/basic/transports.mdx:129` Sending Messages to the Server | SHOULD; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - To cancel, the client SHOULD explicitly send an MCP `CancelledNotification`. |
| `spec/basic/transports.mdx:130` Sending Messages to the Server | MAY; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - To avoid message loss due to disconnection, the server MAY make the stream |
| `spec/basic/transports.mdx:135` Listening for Messages from the Server | MAY; optional client; server | server conforms; optional client unused (get); api/stdio; W2 | 1. The client MAY issue an HTTP GET to the MCP endpoint. This can be used to open an |
| `spec/basic/transports.mdx:138` Listening for Messages from the Server | MUST; optional client; server | server conforms; optional client unused (get); api/stdio; W2 | 2. The client MUST include an `Accept` header, listing `text/event-stream` as a |
| `spec/basic/transports.mdx:140` Listening for Messages from the Server | MUST; optional client; server | server conforms; optional client unused (get); api/stdio; W2 | 3. The server MUST either return `Content-Type: text/event-stream` in response to |
| `spec/basic/transports.mdx:144` Listening for Messages from the Server | MAY; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - The server MAY send JSON-RPC _requests_ and _notifications_ on the stream. |
| `spec/basic/transports.mdx:145` Listening for Messages from the Server | SHOULD; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - These messages SHOULD be unrelated to any concurrently-running JSON-RPC |
| `spec/basic/transports.mdx:147` Listening for Messages from the Server | MUST NOT; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - The server MUST NOT send a JSON-RPC _response_ on the stream unless |
| `spec/basic/transports.mdx:150` Listening for Messages from the Server | MAY; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - The server MAY close the SSE stream at any time. |
| `spec/basic/transports.mdx:152` Listening for Messages from the Server | SHOULD; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | SHOULD follow the same polling behavior as described for POST requests: |
| `spec/basic/transports.mdx:154` Listening for Messages from the Server | MAY; conditional SSE server/client | server inapplicable; general client C3 (sse); mcp/stdio; W2/W3 | - The client MAY close the SSE stream at any time. |
| `spec/basic/transports.mdx:158` Multiple Connections | MAY; SSE sender/receiver | inapplicable with reason (connections); mcp; W2 | 1. The client MAY remain connected to multiple SSE streams simultaneously. |
| `spec/basic/transports.mdx:159` Multiple Connections | MUST; SSE sender/receiver | inapplicable with reason (connections); mcp; W2 | 2. The server MUST send each of its JSON-RPC messages on only one of the connected |
| `spec/basic/transports.mdx:160` Multiple Connections | MUST NOT; SSE sender/receiver | inapplicable with reason (connections); mcp; W2 | streams; that is, it MUST NOT broadcast the same message across multiple streams. |
| `spec/basic/transports.mdx:161` Multiple Connections | MAY; SSE sender/receiver | inapplicable with reason (connections); mcp; W2 | - The risk of message loss MAY be mitigated by making the stream |
| `spec/basic/transports.mdx:169` Resumability and Redelivery | MAY; SSE sender/receiver | server inapplicable; general client C3 (resume); mcp/stdio; W2 | 1. Servers MAY attach an `id` field to their SSE events, as described in the |
| `spec/basic/transports.mdx:171` Resumability and Redelivery | MUST; SSE sender/receiver | server inapplicable; general client C3 (resume); mcp/stdio; W2 | - If present, the ID MUST be globally unique across all streams within that |
| `spec/basic/transports.mdx:174` Resumability and Redelivery | SHOULD; SSE sender/receiver | server inapplicable; general client C3 (resume); mcp/stdio; W2 | - Event IDs SHOULD encode sufficient information to identify the originating |
| `spec/basic/transports.mdx:177` Resumability and Redelivery | SHOULD; SSE sender/receiver | server inapplicable; general client C3 (resume); mcp/stdio; W2 | or server-initiated closure), it SHOULD issue an HTTP GET to the MCP endpoint, |
| `spec/basic/transports.mdx:181` Resumability and Redelivery | MAY; SSE sender/receiver | server inapplicable; general client C3 (resume); mcp/stdio; W2 | - The server MAY use this header to replay messages that would have been sent |
| `spec/basic/transports.mdx:184` Resumability and Redelivery | MUST NOT; SSE sender/receiver | server inapplicable; general client C3 (resume); mcp/stdio; W2 | - The server MUST NOT replay messages that would have been delivered on a |
| `spec/basic/transports.mdx:198` Session Management | MAY; optional stateful server; conditional client | paired server inapplicable; external client C3 (session); mcp/stdio; W1/W2/W3 | 1. A server using the Streamable HTTP transport MAY assign a session ID at |
| `spec/basic/transports.mdx:201` Session Management | SHOULD; optional stateful server; conditional client | paired server inapplicable; external client C3 (session); mcp/stdio; W1/W2/W3 | - The session ID SHOULD be globally unique and cryptographically secure (e.g., a |
| `spec/basic/transports.mdx:203` Session Management | MUST; optional stateful server; conditional client | paired server inapplicable; external client C3 (session); mcp/stdio; W1/W2/W3 | - The session ID MUST only contain visible ASCII characters (ranging from 0x21 to |
| `spec/basic/transports.mdx:205` Session Management | MUST; optional stateful server; conditional client | paired server inapplicable; external client C3 (session); mcp/stdio; W1/W2/W3 | - The client MUST handle the session ID in a secure manner, see Session Hijacking mitigations for more details. |
| `spec/basic/transports.mdx:207` Session Management | MUST; optional stateful server; conditional client | paired server inapplicable; external client C3 (session); mcp/stdio; W1/W2/W3 | the Streamable HTTP transport MUST include it in the `MCP-Session-Id` header on |
| `spec/basic/transports.mdx:209` Session Management | SHOULD; optional stateful server; conditional client | paired server inapplicable; external client C3 (session); mcp/stdio; W1/W2/W3 | - Servers that require a session ID SHOULD respond to requests without an |
| `spec/basic/transports.mdx:211` Session Management | MAY/MUST; optional stateful server; conditional client | paired server inapplicable; external client C3 (session); mcp/stdio; W1/W2/W3 | 3. The server MAY terminate the session at any time, after which it MUST respond |
| `spec/basic/transports.mdx:214` Session Management | MUST; optional stateful server; conditional client | paired server inapplicable; external client C3 (session); mcp/stdio; W1/W2/W3 | `MCP-Session-Id`, it MUST start a new session by sending a new `InitializeRequest` |
| `spec/basic/transports.mdx:217` Session Management | SHOULD; optional stateful server; conditional client | paired server inapplicable; external client C3 (session); mcp/stdio; W1/W2/W3 | the client application) SHOULD send an HTTP DELETE to the MCP endpoint with the |
| `spec/basic/transports.mdx:219` Session Management | MAY; optional stateful server; conditional client | paired server inapplicable; external client C3 (session); mcp/stdio; W1/W2/W3 | - The server MAY respond to this request with HTTP 405 Method Not Allowed, |
| `spec/basic/transports.mdx:265` Protocol Version Header | MUST; HTTP client/server | conforms; non-POST refusal repaired (header); mcp/api/stdio; W1/W2/W3 | If using HTTP, the client MUST include the `MCP-Protocol-Version: |
| `spec/basic/transports.mdx:271` Protocol Version Header | SHOULD; HTTP client/server | conforms; non-POST refusal repaired (header); mcp/api/stdio; W1/W2/W3 | The protocol version sent by the client SHOULD be the one negotiated during |
| `spec/basic/transports.mdx:276` Protocol Version Header | SHOULD; HTTP client/server | conforms; non-POST refusal repaired (header); mcp/api/stdio; W1/W2/W3 | protocol version negotiated during initialization - the server SHOULD assume protocol |
| `spec/basic/transports.mdx:280` Protocol Version Header | MUST; HTTP client/server | conforms; non-POST refusal repaired (header); mcp/api/stdio; W1/W2/W3 | `MCP-Protocol-Version`, it MUST respond with `400 Bad Request`. |
| `spec/basic/transports.mdx:313` Custom Transports | MAY; optional custom transport implementer | inapplicable with reason (custom); mcp/stdio; W1/W2 | Clients and servers MAY implement additional custom transport mechanisms to suit |
| `spec/basic/transports.mdx:317` Custom Transports | MUST; optional custom transport implementer | inapplicable with reason (custom); mcp/stdio; W1/W2 | Implementers who choose to support custom transports MUST ensure they preserve the |
| `spec/basic/transports.mdx:319` Custom Transports | SHOULD; optional custom transport implementer | inapplicable with reason (custom); mcp/stdio; W1/W2 | SHOULD document their specific connection establishment and message exchange patterns |
| `spec/server/tools.mdx:24` User Interaction Model | SHOULD; user application | client-owned SHOULD; server UI inapplicable (ui); mcp/stdio; W7 | For trust & safety and security, there SHOULD always |
| `spec/server/tools.mdx:27` User Interaction Model | SHOULD; user application | client-owned SHOULD; server UI inapplicable (ui); mcp/stdio; W7 | Applications SHOULD: |
| `spec/server/tools.mdx:38` Capabilities | MUST; tool server | conforms (capability); mcp; W1 | Servers that support tools MUST declare the `tools` capability: |
| `spec/server/tools.mdx:153` List Changed Notification | SHOULD; server declaring listChanged | inapplicable with reason (list-change); mcp; W7 | capability SHOULD send a notification: |
| `spec/server/tools.mdx:201` Tool | MUST; schema producer/server | conforms (input-schema); mcp; W7 | - MUST be a valid JSON Schema object (not `null`) |
| `spec/server/tools.mdx:213` Tool | MUST; tool client | opaque bridge conforms; model trust client-owned (annotations); mcp/stdio; W7 | For trust & safety and security, clients MUST consider tool annotations to |
| `spec/server/tools.mdx:219` Tool Names | SHOULD; tool server/client | conforms (names); mcp; W7 | - Tool names SHOULD be between 1 and 128 characters in length (inclusive). |
| `spec/server/tools.mdx:220` Tool Names | SHOULD; tool server/client | conforms (names); mcp; W7 | - Tool names SHOULD be considered case-sensitive. |
| `spec/server/tools.mdx:221` Tool Names | SHOULD; tool server/client | conforms (names); mcp; W7 | - The following SHOULD be the only allowed characters: uppercase and lowercase ASCII letters (A-Z, a-z), digits |
| `spec/server/tools.mdx:223` Tool Names | SHOULD NOT; tool server/client | conforms (names); mcp; W7 | - Tool names SHOULD NOT contain spaces, commas, or other special characters. |
| `spec/server/tools.mdx:224` Tool Names | SHOULD; tool server/client | conforms (names); mcp; W7 | - Tool names SHOULD be unique within a server. |
| `spec/server/tools.mdx:279` Resource Links | MAY; server choosing resource content | inapplicable with reason (resource); mcp; W7 | A tool MAY return links to [Resources, to provide additional context |
| `spec/server/tools.mdx:301` Embedded Resources | MAY; server choosing resource content | inapplicable with reason (resource); mcp; W7 | Resources MAY be embedded to provide additional context |
| `spec/server/tools.mdx:302` Embedded Resources | SHOULD; server choosing resource content | inapplicable with reason (resource); mcp; W7 | or data using a suitable URI scheme. Servers that use embedded resources SHOULD implement the `resources` capability: |
| `spec/server/tools.mdx:326` Structured Content | SHOULD; server choosing structured output/schema | inapplicable with reason (structured); mcp; W7 | For backwards compatibility, a tool that returns structured content SHOULD also return the serialized JSON in a TextContent block. |
| `spec/server/tools.mdx:340` Output Schema | MUST; server choosing structured output/schema | inapplicable with reason (structured); mcp; W7 | - Servers MUST provide structured results that conform to this schema. |
| `spec/server/tools.mdx:341` Output Schema | SHOULD; server choosing structured output/schema | inapplicable with reason (structured); mcp; W7 | - Clients SHOULD validate structured results against this schema. |
| `spec/server/tools.mdx:476` Error Handling | SHOULD; model/UI client | application-owned; bridge timeout conforms (client-tools); stdio; W7/W8 | Clients SHOULD provide tool execution errors to language models to enable self-correction. |
| `spec/server/tools.mdx:477` Error Handling | MAY; model/UI client | application-owned; bridge timeout conforms (client-tools); stdio; W7/W8 | Clients MAY provide protocol errors to language models, though these are less likely to result in successful recovery. |
| `spec/server/tools.mdx:512` Security Considerations | MUST; tool server | conforms; rate repaired; four child rows below (server-tools); mcp/api; W6/W7/W9 | 1. Servers MUST: |
| `spec/server/tools.mdx:518` Security Considerations | SHOULD; model/UI client | application-owned; bridge timeout conforms (client-tools); stdio; W7/W8 | 2. Clients SHOULD: |

### Governing-list child duties and non-keyword shapes

These expand governing keywords above rather than hide compound requirements.

| Revision/source | Level / actor | Disposition, applicability, owner and proof |
| --- | --- | --- |
| 2025-03-26 `spec/server/tools.mdx` Security / validate all tool inputs | MUST; server | Conforms: required/type/null/unknown-key/domain checks stage before effects. Closed schema vocabulary, wait timeout retains accepted isError split. mcp::tool_call/arg_*; W7/W9. |
| 2025-03-26 `spec/server/tools.mdx` Security / proper access controls | MUST; server | Conforms at the development trust boundary: default local bind plus whole-plane Host/Origin admission; explicit off-loopback exposure delegates network admission to operator. No per-inbox identity or Internet OAuth claim. 0030 §2/0043 §§1–2; W6. Hints confer no privilege. |
| 2025-03-26 `spec/server/tools.mdx` Security / rate limit invocations | MUST; server | Repaired: shared per-router monotonic fixed window 1024 tools/call attempts/second, including notifications/March batch members. Excess isError/silent refusal before effects. No peer-key map, timer task or retry. A caller can exhaust shared capacity; adjacent windows permit boundary bursts; active waits/REST/SMTP are not limited. ToolRate/api router; W9. |
| 2025-03-26 `spec/server/tools.mdx` Security / sanitize outputs | MUST; server | Conforms for actual text-data context: typed field projection, raw excluded, TextContent only, mail encoded inside JSON object strings, control bytes escaped, no resource or HTML rendering, shell evaluation or URL fetching. Error envelopes are constructed, never parsed from mail. Sender-honest text/HTML survive decoding (0031 §1/0005). W7 tests markup/newline/ESC/envelope-looking data cannot add a block/RPC message. Clients must preserve untrusted-data boundaries; no prompt-injection immunity claim. |
| 2025-03-26 `spec/server/tools.mdx` Security / confirmation, input display, validation, timeouts, audit | SHOULD; model client | Application-owned UI/LLM policy; headless bridge has timeouts and no model execution. W8. |
| 2025-03-26 `spec/basic/lifecycle.mdx` Operation / negotiated version and capabilities | SHOULD; both | Conforms negotiated operations: immutable version/header, tools-only server output, no task/client features. W1/W3/W5/W7; invalid legacy forwarding C4. |
| 2025-03-26 `spec/basic/transports.mdx` stdio / message kinds | Shape; senders | March permits batches, June/November individual-only. W5; semantic-invalid forwarding C6 remains. |
| 2025-03-26 `spec/server/tools.mdx` listing/calling/text/pagination | Shape; server | Complete static twelve-tool page with no nextCursor. No pagination is required when all results fit one page. CallToolResult content array/text strings and boolean isError. W1/W7; C5 error guidance departure explicit. |
| 2025-06-18 `spec/server/tools.mdx` Security / validate all tool inputs | MUST; server | Conforms: required/type/null/unknown-key/domain checks stage before effects. Closed schema vocabulary, wait timeout retains accepted isError split. mcp::tool_call/arg_*; W7/W9. |
| 2025-06-18 `spec/server/tools.mdx` Security / proper access controls | MUST; server | Conforms at the development trust boundary: default local bind plus whole-plane Host/Origin admission; explicit off-loopback exposure delegates network admission to operator. No per-inbox identity or Internet OAuth claim. 0030 §2/0043 §§1–2; W6. Hints confer no privilege. |
| 2025-06-18 `spec/server/tools.mdx` Security / rate limit invocations | MUST; server | Repaired: shared per-router monotonic fixed window 1024 tools/call attempts/second, including notifications/March batch members. Excess isError/silent refusal before effects. No peer-key map, timer task or retry. A caller can exhaust shared capacity; adjacent windows permit boundary bursts; active waits/REST/SMTP are not limited. ToolRate/api router; W9. |
| 2025-06-18 `spec/server/tools.mdx` Security / sanitize outputs | MUST; server | Conforms for actual text-data context: typed field projection, raw excluded, TextContent only, mail encoded inside JSON object strings, control bytes escaped, no resource or HTML rendering, shell evaluation or URL fetching. Error envelopes are constructed, never parsed from mail. Sender-honest text/HTML survive decoding (0031 §1/0005). W7 tests markup/newline/ESC/envelope-looking data cannot add a block/RPC message. Clients must preserve untrusted-data boundaries; no prompt-injection immunity claim. |
| 2025-06-18 `spec/server/tools.mdx` Security / confirmation, input display, validation, timeouts, audit | SHOULD; model client | Application-owned UI/LLM policy; headless bridge has timeouts and no model execution. W8. |
| 2025-06-18 `spec/basic/lifecycle.mdx` Operation / negotiated version and capabilities | MUST; both | Conforms negotiated operations: immutable version/header, tools-only server output, no task/client features. W1/W3/W5/W7; invalid legacy forwarding C4. |
| 2025-06-18 `spec/basic/transports.mdx` stdio / message kinds | Shape; senders | March permits batches, June/November individual-only. W5; semantic-invalid forwarding C6 remains. |
| 2025-06-18 `spec/server/tools.mdx` listing/calling/text/pagination | Shape; server | Complete static twelve-tool page with no nextCursor. No pagination is required when all results fit one page. CallToolResult content array/text strings and boolean isError. W1/W7; C5 error guidance departure explicit. |
| 2025-11-25 `spec/server/tools.mdx` Security / validate all tool inputs | MUST; server | Conforms: required/type/null/unknown-key/domain checks stage before effects. Closed schema vocabulary, wait timeout retains accepted isError split. mcp::tool_call/arg_*; W7/W9. |
| 2025-11-25 `spec/server/tools.mdx` Security / proper access controls | MUST; server | Conforms at the development trust boundary: default local bind plus whole-plane Host/Origin admission; explicit off-loopback exposure delegates network admission to operator. No per-inbox identity or Internet OAuth claim. 0030 §2/0043 §§1–2; W6. Hints confer no privilege. |
| 2025-11-25 `spec/server/tools.mdx` Security / rate limit invocations | MUST; server | Repaired: shared per-router monotonic fixed window 1024 tools/call attempts/second, including notifications/March batch members. Excess isError/silent refusal before effects. No peer-key map, timer task or retry. A caller can exhaust shared capacity; adjacent windows permit boundary bursts; active waits/REST/SMTP are not limited. ToolRate/api router; W9. |
| 2025-11-25 `spec/server/tools.mdx` Security / sanitize outputs | MUST; server | Conforms for actual text-data context: typed field projection, raw excluded, TextContent only, mail encoded inside JSON object strings, control bytes escaped, no resource or HTML rendering, shell evaluation or URL fetching. Error envelopes are constructed, never parsed from mail. Sender-honest text/HTML survive decoding (0031 §1/0005). W7 tests markup/newline/ESC/envelope-looking data cannot add a block/RPC message. Clients must preserve untrusted-data boundaries; no prompt-injection immunity claim. |
| 2025-11-25 `spec/server/tools.mdx` Security / confirmation, input display, validation, timeouts, audit | SHOULD; model client | Application-owned UI/LLM policy; headless bridge has timeouts and no model execution. W8. |
| 2025-11-25 `spec/basic/lifecycle.mdx` Operation / negotiated version and capabilities | MUST; both | Conforms negotiated operations: immutable version/header, tools-only server output, no task/client features. W1/W3/W5/W7; invalid legacy forwarding C4. |
| 2025-11-25 `spec/basic/transports.mdx` stdio / message kinds | Shape; senders | March permits batches, June/November individual-only. W5; semantic-invalid forwarding C6 remains. |
| 2025-11-25 `spec/server/tools.mdx` listing/calling/text/pagination | Shape; server | Complete static twelve-tool page with no nextCursor. No pagination is required when all results fit one page. CallToolResult content array/text strings and boolean isError. W1/W7; C5 error guidance departure explicit. |

## TypeScript and generated JSON schema audit

Both formats were inspected independently. TS number vs generated JSON integer
for IDs is a real distinction (C2). November renames success/error interfaces,
makes error ID optional and uses $defs; earlier generated schemas use definitions.
Required lists below come from JSON; inherited JSON-RPC envelopes still apply to
all derived requests/notifications. Optional titles/icons/instructions/metadata,
other content variants, output schemas and tasks are absent, not advertised.
Tool input schemas are values carried in messages, distinct from message schemas.

Owned input schemas use {type object/string/integer, properties, required,
additionalProperties false, enum, minimum, maximum, default, description}.
Each is an object, properties map names to schema objects, required entries
name properties, integer bounds/defaults are integers and string enums strings.
These are valid 2020-12 keywords/shapes. W7 checks every descriptor's vocabulary;
staging suites exercise supplied/default/null/type/range semantics. No $ref,
remote schema or schema-valued input is consumed, so remote unsupported-dialect
handling has a false antecedent. No generic validator or additional dialect is
claimed. An undeclared '$schema' tool argument is an unknown-argument refusal,
not a schema-validation demonstration. Clients consume the published schemas.

| Revision | TS source symbol/line; JSON pointer | Required/shape | Disposition; owner; witness |
| --- | --- | --- | --- |
| 2025-03-26 `schema/schema.ts:37` Request; `schema/schema.json#/definitions/Request` | required: method; fields: method, params | conforms valid input; C4 invalid forwarding; mcp/stdio; W1/W5 |
| 2025-03-26 `schema/schema.ts:50` Notification; `schema/schema.json#/definitions/Notification` | required: method; fields: method, params | conforms valid input; C4 invalid forwarding; mcp/stdio; W1/W5 |
| 2025-03-26 `schema/schema.ts:61` Result; `schema/schema.json#/definitions/Result` | required: (none); fields: _meta | conforms consumption; production inapplicable; mcp/stdio; W7 |
| 2025-03-26 `schema/schema.ts:72` RequestId; `schema/schema.json#/definitions/RequestId` | required: (none); JSON type ['string', 'integer']; TS export type RequestId = string &#124; number; | retained deviation C2; server receiver tolerance retained; mcp/stdio; W1/W4 |
| 2025-03-26 `schema/schema.ts:30` ProgressToken; `schema/schema.json#/definitions/ProgressToken` | required: (none); JSON type ['string', 'integer']; TS export type ProgressToken = string &#124; number; | conforms consumption; production inapplicable; mcp/stdio; W7 |
| 2025-03-26 `schema/schema.ts:35` Cursor; `schema/schema.json#/definitions/Cursor` | required: (none); JSON type string; TS export type Cursor = string; | conforms consumption; production inapplicable; mcp/stdio; W7 |
| 2025-03-26 `schema/schema.ts:77` JSONRPCRequest; `schema/schema.json#/definitions/JSONRPCRequest` | required: id, jsonrpc, method; fields: id, jsonrpc, method, params | retained deviation C2/C4/C6; mcp/stdio; W1/W3/W4 |
| 2025-03-26 `schema/schema.ts:85` JSONRPCNotification; `schema/schema.json#/definitions/JSONRPCNotification` | required: jsonrpc, method; fields: jsonrpc, method, params | conforms for valid notifications; C4 for invalid-input forwarding; mcp/stdio; W1/W3 |
| 2025-03-26 `schema/schema.ts:92` JSONRPCResponse; `schema/schema.json#/definitions/JSONRPCResponse` | required: id, jsonrpc, result; fields: id, jsonrpc, result | conforms for owned valid-ID responses; C6 for forwarded replies; mcp/stdio; W1/W4 |
| 2025-03-26 `schema/schema.ts:108` JSONRPCError; `schema/schema.json#/definitions/JSONRPCError` | required: error, id, jsonrpc; fields: error, id, jsonrpc | conforms for owned valid-ID responses; C6 for forwarded replies; mcp/stdio; W1/W4 |
| 2025-03-26 `schema/schema.ts:6` JSONRPCMessage; `schema/schema.json#/definitions/JSONRPCMessage` | required: (none); union JSONRPCRequest, JSONRPCNotification, inline, JSONRPCResponse, JSONRPCError, inline | conforms for owned valid-ID responses; C6 for forwarded replies; mcp/stdio; W1/W4 |
| 2025-03-26 `schema/schema.ts:17` JSONRPCBatchRequest; `schema/schema.json#/definitions/JSONRPCBatchRequest` | required: (none) | conforms; mcp/stdio; W1/W5 |
| 2025-03-26 `schema/schema.ts:22` JSONRPCBatchResponse; `schema/schema.json#/definitions/JSONRPCBatchResponse` | required: (none) | conforms for owned valid-ID responses; C6 for forwarded replies; mcp/stdio; W1/W4 |
| 2025-03-26 `schema/schema.ts:164` InitializeRequest; `schema/schema.json#/definitions/InitializeRequest` | required: method, params; fields: method, params | C4 for compatibility; conforms valid modern flow; stdio/mcp; W1/W3 |
| 2025-03-26 `schema/schema.ts:179` InitializeResult; `schema/schema.json#/definitions/InitializeResult` | required: capabilities, protocolVersion, serverInfo; fields: _meta, capabilities, instructions, protocolVersion, serverInfo | conforms; required-result installation repaired; mcp/stdio; W1/W3 |
| 2025-03-26 `schema/schema.ts:277` Implementation; `schema/schema.json#/definitions/Implementation` | required: name, version; fields: name, version | conforms; required-result installation repaired; mcp/stdio; W1/W3 |
| 2025-03-26 `schema/schema.ts:205` ClientCapabilities; `schema/schema.json#/definitions/ClientCapabilities` | required: (none); fields: experimental, roots, sampling | conforms negotiated operations; C4 invalid forwarding; mcp/stdio; W1/W3/W5/W7 |
| 2025-03-26 `schema/schema.ts:228` ServerCapabilities; `schema/schema.json#/definitions/ServerCapabilities` | required: (none); fields: completions, experimental, logging, prompts, resources, tools | conforms negotiated operations; C4 invalid forwarding; mcp/stdio; W1/W3/W5/W7 |
| 2025-03-26 `schema/schema.ts:198` InitializedNotification; `schema/schema.json#/definitions/InitializedNotification` | required: method; fields: method, params | conforms for valid notifications; C4 for invalid-input forwarding; mcp/stdio; W1/W3 |
| 2025-03-26 `schema/schema.ts:286` PingRequest; `schema/schema.json#/definitions/PingRequest` | required: method; fields: method, params | conforms for owned valid-ID responses; C6 for forwarded replies; mcp/stdio; W1/W4 |
| 2025-03-26 `schema/schema.ts:131` EmptyResult; `schema/schema.json#/definitions/EmptyResult` | required: (none) | conforms for owned valid-ID responses; C6 for forwarded replies; mcp/stdio; W1/W4 |
| 2025-03-26 `schema/schema.ts:321` PaginatedRequest; `schema/schema.json#/definitions/PaginatedRequest` | required: method; fields: method, params | conforms complete static page; invalid sender envelope C4; mcp/stdio; W7 |
| 2025-03-26 `schema/schema.ts:331` PaginatedResult; `schema/schema.json#/definitions/PaginatedResult` | required: (none); fields: _meta, nextCursor | conforms complete static page; invalid sender envelope C4; mcp/stdio; W7 |
| 2025-03-26 `schema/schema.ts:675` ListToolsRequest; `schema/schema.json#/definitions/ListToolsRequest` | required: method; fields: method, params | conforms complete static page; invalid sender envelope C4; mcp/stdio; W7 |
| 2025-03-26 `schema/schema.ts:682` ListToolsResult; `schema/schema.json#/definitions/ListToolsResult` | required: tools; fields: _meta, nextCursor, tools | conforms complete static page; invalid sender envelope C4; mcp/stdio; W7 |
| 2025-03-26 `schema/schema.ts:784` Tool; `schema/schema.json#/definitions/Tool` | required: inputSchema, name; fields: annotations, description, inputSchema, name | conforms; mcp; W7 |
| 2025-03-26 `schema/schema.ts:737` ToolAnnotations; `schema/schema.json#/definitions/ToolAnnotations` | required: (none); fields: destructiveHint, idempotentHint, openWorldHint, readOnlyHint, title | opaque bridge conforms; model trust client-owned; mcp/stdio; W7 |
| 2025-03-26 `schema/schema.ts:723` ToolListChangedNotification; `schema/schema.json#/definitions/ToolListChangedNotification` | required: method; fields: method, params | inapplicable with reason; mcp; W7 |
| 2025-03-26 `schema/schema.ts:712` CallToolRequest; `schema/schema.json#/definitions/CallToolRequest` | required: method, params; fields: method, params | conforms valid input; C4 invalid forwarding; mcp/stdio; W1/W5 |
| 2025-03-26 `schema/schema.ts:698` CallToolResult; `schema/schema.json#/definitions/CallToolResult` | required: content; fields: _meta, content, isError | conforms owned valid IDs; C5 justified SHOULD departure; #153 absent; mcp/stdio; W7 |
| 2025-03-26 `schema/schema.ts:949` TextContent; `schema/schema.json#/definitions/TextContent` | required: text, type; fields: annotations, text, type | conforms type=text/string text, no active resources; mcp/stdio; W7 |
| 2025-03-26 `schema/schema.ts:924` Annotations; `schema/schema.json#/definitions/Annotations` | required: (none); fields: audience, priority | conforms consumption; production inapplicable; mcp/stdio; W7 |
| 2025-06-18 `schema/schema.ts:31` Request; `schema/schema.json#/definitions/Request` | required: method; fields: method, params | conforms valid input; C4 invalid forwarding; mcp/stdio; W1/W5 |
| 2025-06-18 `schema/schema.ts:49` Notification; `schema/schema.json#/definitions/Notification` | required: method; fields: method, params | conforms valid input; C4 invalid forwarding; mcp/stdio; W1/W5 |
| 2025-06-18 `schema/schema.ts:63` Result; `schema/schema.json#/definitions/Result` | required: (none); fields: _meta | conforms consumption; production inapplicable; mcp/stdio; W7 |
| 2025-06-18 `schema/schema.ts:76` RequestId; `schema/schema.json#/definitions/RequestId` | required: (none); JSON type ['string', 'integer']; TS export type RequestId = string &#124; number; | retained deviation C2; server receiver tolerance retained; mcp/stdio; W1/W4 |
| 2025-06-18 `schema/schema.ts:21` ProgressToken; `schema/schema.json#/definitions/ProgressToken` | required: (none); JSON type ['string', 'integer']; TS export type ProgressToken = string &#124; number; | conforms consumption; production inapplicable; mcp/stdio; W7 |
| 2025-06-18 `schema/schema.ts:28` Cursor; `schema/schema.json#/definitions/Cursor` | required: (none); JSON type string; TS export type Cursor = string; | conforms consumption; production inapplicable; mcp/stdio; W7 |
| 2025-06-18 `schema/schema.ts:83` JSONRPCRequest; `schema/schema.json#/definitions/JSONRPCRequest` | required: id, jsonrpc, method; fields: id, jsonrpc, method, params | retained deviation C2/C4/C6; mcp/stdio; W1/W3/W4 |
| 2025-06-18 `schema/schema.ts:93` JSONRPCNotification; `schema/schema.json#/definitions/JSONRPCNotification` | required: jsonrpc, method; fields: jsonrpc, method, params | conforms for valid notifications; C4 for invalid-input forwarding; mcp/stdio; W1/W3 |
| 2025-06-18 `schema/schema.ts:102` JSONRPCResponse; `schema/schema.json#/definitions/JSONRPCResponse` | required: id, jsonrpc, result; fields: id, jsonrpc, result | conforms for owned valid-ID responses; C6 for forwarded replies; mcp/stdio; W1/W4 |
| 2025-06-18 `schema/schema.ts:120` JSONRPCError; `schema/schema.json#/definitions/JSONRPCError` | required: error, id, jsonrpc; fields: error, id, jsonrpc | conforms for owned valid-ID responses; C6 for forwarded replies; mcp/stdio; W1/W4 |
| 2025-06-18 `schema/schema.ts:8` JSONRPCMessage; `schema/schema.json#/definitions/JSONRPCMessage` | required: (none); union JSONRPCRequest, JSONRPCNotification, JSONRPCResponse, JSONRPCError | conforms for owned valid-ID responses; C6 for forwarded replies; mcp/stdio; W1/W4 |
| 2025-06-18 `schema/schema.ts:182` InitializeRequest; `schema/schema.json#/definitions/InitializeRequest` | required: method, params; fields: method, params | C4 for compatibility; conforms valid modern flow; stdio/mcp; W1/W3 |
| 2025-06-18 `schema/schema.ts:199` InitializeResult; `schema/schema.json#/definitions/InitializeResult` | required: capabilities, protocolVersion, serverInfo; fields: _meta, capabilities, instructions, protocolVersion, serverInfo | conforms; required-result installation repaired; mcp/stdio; W1/W3 |
| 2025-06-18 `schema/schema.ts:331` Implementation; `schema/schema.json#/definitions/Implementation` | required: name, version; fields: name, title, version | conforms; required-result installation repaired; mcp/stdio; W1/W3 |
| 2025-06-18 `schema/schema.ts:309` BaseMetadata; `schema/schema.json#/definitions/BaseMetadata` | required: name; fields: name, title | conforms; required-result installation repaired; mcp/stdio; W1/W3 |
| 2025-06-18 `schema/schema.ts:229` ClientCapabilities; `schema/schema.json#/definitions/ClientCapabilities` | required: (none); fields: elicitation, experimental, roots, sampling | conforms negotiated operations; C4 invalid forwarding; mcp/stdio; W1/W3/W5/W7 |
| 2025-06-18 `schema/schema.ts:258` ServerCapabilities; `schema/schema.json#/definitions/ServerCapabilities` | required: (none); fields: completions, experimental, logging, prompts, resources, tools | conforms negotiated operations; C4 invalid forwarding; mcp/stdio; W1/W3/W5/W7 |
| 2025-06-18 `schema/schema.ts:220` InitializedNotification; `schema/schema.json#/definitions/InitializedNotification` | required: method; fields: method, params | conforms for valid notifications; C4 for invalid-input forwarding; mcp/stdio; W1/W3 |
| 2025-06-18 `schema/schema.ts:341` PingRequest; `schema/schema.json#/definitions/PingRequest` | required: method; fields: method, params | conforms for owned valid-ID responses; C6 for forwarded replies; mcp/stdio; W1/W4 |
| 2025-06-18 `schema/schema.ts:145` EmptyResult; `schema/schema.json#/definitions/EmptyResult` | required: (none) | conforms for owned valid-ID responses; C6 for forwarded replies; mcp/stdio; W1/W4 |
| 2025-06-18 `schema/schema.ts:379` PaginatedRequest; `schema/schema.json#/definitions/PaginatedRequest` | required: method; fields: method, params | conforms complete static page; invalid sender envelope C4; mcp/stdio; W7 |
| 2025-06-18 `schema/schema.ts:390` PaginatedResult; `schema/schema.json#/definitions/PaginatedResult` | required: (none); fields: _meta, nextCursor | conforms complete static page; invalid sender envelope C4; mcp/stdio; W7 |
| 2025-06-18 `schema/schema.ts:801` ListToolsRequest; `schema/schema.json#/definitions/ListToolsRequest` | required: method; fields: method, params | conforms complete static page; invalid sender envelope C4; mcp/stdio; W7 |
| 2025-06-18 `schema/schema.ts:810` ListToolsResult; `schema/schema.json#/definitions/ListToolsResult` | required: tools; fields: _meta, nextCursor, tools | conforms complete static page; invalid sender envelope C4; mcp/stdio; W7 |
| 2025-06-18 `schema/schema.ts:930` Tool; `schema/schema.json#/definitions/Tool` | required: inputSchema, name; fields: _meta, annotations, description, inputSchema, name, outputSchema, title | conforms; mcp; W7 |
| 2025-06-18 `schema/schema.ts:881` ToolAnnotations; `schema/schema.json#/definitions/ToolAnnotations` | required: (none); fields: destructiveHint, idempotentHint, openWorldHint, readOnlyHint, title | opaque bridge conforms; model trust client-owned; mcp/stdio; W7 |
| 2025-06-18 `schema/schema.ts:865` ToolListChangedNotification; `schema/schema.json#/definitions/ToolListChangedNotification` | required: method; fields: method, params | inapplicable with reason; mcp; W7 |
| 2025-06-18 `schema/schema.ts:852` CallToolRequest; `schema/schema.json#/definitions/CallToolRequest` | required: method, params; fields: method, params | conforms valid input; C4 invalid forwarding; mcp/stdio; W1/W5 |
| 2025-06-18 `schema/schema.ts:819` CallToolResult; `schema/schema.json#/definitions/CallToolResult` | required: content; fields: _meta, content, isError, structuredContent | conforms owned valid IDs; C5 justified SHOULD departure; #153 absent; mcp/stdio; W7 |
| 2025-06-18 `schema/schema.ts:1141` TextContent; `schema/schema.json#/definitions/TextContent` | required: text, type; fields: _meta, annotations, text, type | conforms type=text/string text, no active resources; mcp/stdio; W7 |
| 2025-06-18 `schema/schema.ts:1133` ContentBlock; `schema/schema.json#/definitions/ContentBlock` | required: (none); union TextContent, ImageContent, AudioContent, ResourceLink, EmbeddedResource | conforms TextContent alternative only; other variants unused; mcp/stdio; W7 |
| 2025-06-18 `schema/schema.ts:1098` Annotations; `schema/schema.json#/definitions/Annotations` | required: (none); fields: audience, lastModified, priority | conforms consumption; production inapplicable; mcp/stdio; W7 |
| 2025-11-25 `schema/schema.ts:65` Request; `schema/schema.json#/$defs/Request` | required: method; fields: method, params | conforms valid input; C4 invalid forwarding; mcp/stdio; W1/W5 |
| 2025-11-25 `schema/schema.ts:51` RequestParams; `schema/schema.json#/$defs/RequestParams` | required: (none); fields: _meta | conforms valid input; C4 invalid forwarding; mcp/stdio; W1/W5 |
| 2025-11-25 `schema/schema.ts:81` Notification; `schema/schema.json#/$defs/Notification` | required: method; fields: method, params | conforms valid input; C4 invalid forwarding; mcp/stdio; W1/W5 |
| 2025-11-25 `schema/schema.ts:73` NotificationParams; `schema/schema.json#/$defs/NotificationParams` | required: (none); fields: _meta | conforms valid input; C4 invalid forwarding; mcp/stdio; W1/W5 |
| 2025-11-25 `schema/schema.ts:91` Result; `schema/schema.json#/$defs/Result` | required: (none); fields: _meta | conforms consumption; production inapplicable; mcp/stdio; W7 |
| 2025-11-25 `schema/schema.ts:122` RequestId; `schema/schema.json#/$defs/RequestId` | required: (none); JSON type ['string', 'integer']; TS export type RequestId = string &#124; number; | retained deviation C2; server receiver tolerance retained; mcp/stdio; W1/W4 |
| 2025-11-25 `schema/schema.ts:21` ProgressToken; `schema/schema.json#/$defs/ProgressToken` | required: (none); JSON type ['string', 'integer']; TS export type ProgressToken = string &#124; number; | conforms consumption; production inapplicable; mcp/stdio; W7 |
| 2025-11-25 `schema/schema.ts:28` Cursor; `schema/schema.json#/$defs/Cursor` | required: (none); JSON type string; TS export type Cursor = string; | conforms consumption; production inapplicable; mcp/stdio; W7 |
| 2025-11-25 `schema/schema.ts:129` JSONRPCRequest; `schema/schema.json#/$defs/JSONRPCRequest` | required: id, jsonrpc, method; fields: id, jsonrpc, method, params | retained deviation C2/C4/C6; mcp/stdio; W1/W3/W4 |
| 2025-11-25 `schema/schema.ts:139` JSONRPCNotification; `schema/schema.json#/$defs/JSONRPCNotification` | required: jsonrpc, method; fields: jsonrpc, method, params | conforms for valid notifications; C4 for invalid-input forwarding; mcp/stdio; W1/W3 |
| 2025-11-25 `schema/schema.ts:170` JSONRPCResponse; `schema/schema.json#/$defs/JSONRPCResponse` | required: (none); union JSONRPCResultResponse, JSONRPCErrorResponse | conforms for owned valid-ID responses; C6 for forwarded replies; mcp/stdio; W1/W4 |
| 2025-11-25 `schema/schema.ts:102` Error; `schema/schema.json#/$defs/Error` | required: code, message; fields: code, data, message | conforms for owned valid-ID responses; C6 for forwarded replies; mcp/stdio; W1/W4 |
| 2025-11-25 `schema/schema.ts:148` JSONRPCResultResponse; `schema/schema.json#/$defs/JSONRPCResultResponse` | required: id, jsonrpc, result; fields: id, jsonrpc, result | conforms for owned valid-ID responses; C6 for forwarded replies; mcp/stdio; W1/W4 |
| 2025-11-25 `schema/schema.ts:159` JSONRPCErrorResponse; `schema/schema.json#/$defs/JSONRPCErrorResponse` | required: error, jsonrpc; fields: error, id, jsonrpc | conforms for owned valid-ID responses; C6 for forwarded replies; mcp/stdio; W1/W4 |
| 2025-11-25 `schema/schema.ts:8` JSONRPCMessage; `schema/schema.json#/$defs/JSONRPCMessage` | required: (none); union JSONRPCRequest, JSONRPCNotification, JSONRPCResultResponse, JSONRPCErrorResponse | conforms for owned valid-ID responses; C6 for forwarded replies; mcp/stdio; W1/W4 |
| 2025-11-25 `schema/schema.ts:269` InitializeRequest; `schema/schema.json#/$defs/InitializeRequest` | required: id, jsonrpc, method, params; fields: id, jsonrpc, method, params | C4 for compatibility; conforms valid modern flow; stdio/mcp; W1/W3 |
| 2025-11-25 `schema/schema.ts:255` InitializeRequestParams; `schema/schema.json#/$defs/InitializeRequestParams` | required: capabilities, clientInfo, protocolVersion; fields: _meta, capabilities, clientInfo, protocolVersion | C4 for compatibility; conforms valid modern flow; stdio/mcp; W1/W3 |
| 2025-11-25 `schema/schema.ts:279` InitializeResult; `schema/schema.json#/$defs/InitializeResult` | required: capabilities, protocolVersion, serverInfo; fields: _meta, capabilities, instructions, protocolVersion, serverInfo | conforms; required-result installation repaired; mcp/stdio; W1/W3 |
| 2025-11-25 `schema/schema.ts:550` Implementation; `schema/schema.json#/$defs/Implementation` | required: name, version; fields: description, icons, name, title, version, websiteUrl | conforms; required-result installation repaired; mcp/stdio; W1/W3 |
| 2025-11-25 `schema/schema.ts:528` BaseMetadata; `schema/schema.json#/$defs/BaseMetadata` | required: name; fields: name, title | conforms; required-result installation repaired; mcp/stdio; W1/W3 |
| 2025-11-25 `schema/schema.ts:310` ClientCapabilities; `schema/schema.json#/$defs/ClientCapabilities` | required: (none); fields: elicitation, experimental, roots, sampling, tasks | conforms negotiated operations; C4 invalid forwarding; mcp/stdio; W1/W3/W5/W7 |
| 2025-11-25 `schema/schema.ts:386` ServerCapabilities; `schema/schema.json#/$defs/ServerCapabilities` | required: (none); fields: completions, experimental, logging, prompts, resources, tasks, tools | conforms negotiated operations; C4 invalid forwarding; mcp/stdio; W1/W3/W5/W7 |
| 2025-11-25 `schema/schema.ts:300` InitializedNotification; `schema/schema.json#/$defs/InitializedNotification` | required: jsonrpc, method; fields: jsonrpc, method, params | conforms for valid notifications; C4 for invalid-input forwarding; mcp/stdio; W1/W3 |
| 2025-11-25 `schema/schema.ts:576` PingRequest; `schema/schema.json#/$defs/PingRequest` | required: id, jsonrpc, method; fields: id, jsonrpc, method, params | conforms for owned valid-ID responses; C6 for forwarded replies; mcp/stdio; W1/W4 |
| 2025-11-25 `schema/schema.ts:207` EmptyResult; `schema/schema.json#/$defs/EmptyResult` | required: (none) | conforms for owned valid-ID responses; C6 for forwarded replies; mcp/stdio; W1/W4 |
| 2025-11-25 `schema/schema.ts:636` PaginatedRequest; `schema/schema.json#/$defs/PaginatedRequest` | required: id, jsonrpc, method; fields: id, jsonrpc, method, params | conforms complete static page; invalid sender envelope C4; mcp/stdio; W7 |
| 2025-11-25 `schema/schema.ts:641` PaginatedResult; `schema/schema.json#/$defs/PaginatedResult` | required: (none); fields: _meta, nextCursor | conforms complete static page; invalid sender envelope C4; mcp/stdio; W7 |
| 2025-11-25 `schema/schema.ts:1086` ListToolsRequest; `schema/schema.json#/$defs/ListToolsRequest` | required: id, jsonrpc, method; fields: id, jsonrpc, method, params | conforms complete static page; invalid sender envelope C4; mcp/stdio; W7 |
| 2025-11-25 `schema/schema.ts:1095` ListToolsResult; `schema/schema.json#/$defs/ListToolsResult` | required: tools; fields: _meta, nextCursor, tools | conforms complete static page; invalid sender envelope C4; mcp/stdio; W7 |
| 2025-11-25 `schema/schema.ts:1249` Tool; `schema/schema.json#/$defs/Tool` | required: inputSchema, name; fields: _meta, annotations, description, execution, icons, inputSchema, name, outputSchema, title | conforms; mcp; W7 |
| 2025-11-25 `schema/schema.ts:1180` ToolAnnotations; `schema/schema.json#/$defs/ToolAnnotations` | required: (none); fields: destructiveHint, idempotentHint, openWorldHint, readOnlyHint, title | opaque bridge conforms; model trust client-owned; mcp/stdio; W7 |
| 2025-11-25 `schema/schema.ts:1229` ToolExecution; `schema/schema.json#/$defs/ToolExecution` | required: (none); fields: taskSupport | inapplicable: no tasks/execution metadata, default forbidden; invalid senders C4; mcp/stdio; W7 |
| 2025-11-25 `schema/schema.ts:1163` ToolListChangedNotification; `schema/schema.json#/$defs/ToolListChangedNotification` | required: jsonrpc, method; fields: jsonrpc, method, params | inapplicable with reason; mcp; W7 |
| 2025-11-25 `schema/schema.ts:1153` CallToolRequest; `schema/schema.json#/$defs/CallToolRequest` | required: id, jsonrpc, method, params; fields: id, jsonrpc, method, params | conforms valid input; C4 invalid forwarding; mcp/stdio; W1/W5 |
| 2025-11-25 `schema/schema.ts:1137` CallToolRequestParams; `schema/schema.json#/$defs/CallToolRequestParams` | required: name; fields: _meta, arguments, name, task | conforms valid input; C4 invalid forwarding; mcp/stdio; W1/W5 |
| 2025-11-25 `schema/schema.ts:1104` CallToolResult; `schema/schema.json#/$defs/CallToolResult` | required: content; fields: _meta, content, isError, structuredContent | conforms owned valid IDs; C5 justified SHOULD departure; #153 absent; mcp/stdio; W7 |
| 2025-11-25 `schema/schema.ts:1748` TextContent; `schema/schema.json#/$defs/TextContent` | required: text, type; fields: _meta, annotations, text, type | conforms type=text/string text, no active resources; mcp/stdio; W7 |
| 2025-11-25 `schema/schema.ts:1740` ContentBlock; `schema/schema.json#/$defs/ContentBlock` | required: (none); union TextContent, ImageContent, AudioContent, ResourceLink, EmbeddedResource | conforms TextContent alternative only; other variants unused; mcp/stdio; W7 |
| 2025-11-25 `schema/schema.ts:35` TaskAugmentedRequestParams; `schema/schema.json#/$defs/TaskAugmentedRequestParams` | required: (none); fields: _meta, task | inapplicable: no tasks/execution metadata, default forbidden; invalid senders C4; mcp/stdio; W7 |
| 2025-11-25 `schema/schema.ts:508` Icons; `schema/schema.json#/$defs/Icons` | required: (none); fields: icons | inapplicable with reason; mcp/stdio; W7 |
| 2025-11-25 `schema/schema.ts:464` Icon; `schema/schema.json#/$defs/Icon` | required: src; fields: mimeType, sizes, src, theme | inapplicable with reason; mcp/stdio; W7 |
| 2025-11-25 `schema/schema.ts:1705` Annotations; `schema/schema.json#/$defs/Annotations` | required: (none); fields: audience, lastModified, priority | conforms consumption; production inapplicable; mcp/stdio; W7 |

Annotations have identical defaults across revisions: readOnly false,
destructive true (for non-read-only tools), idempotent false, openWorld true.
March may omit hints; modern six reads/mutations carry the directed hints exactly.
Optional title precedence is not consumed by the bridge. November omitted
ToolExecution means taskSupport forbidden, not optional/required. No task request
is initiated. Owned error codes/messages meet integer/string independently of C2.
March Transports item 5's “one JSON object” wording is read with its Basic batch
requirement and authoritative JSONRPCBatchResponse union: arrays remain arrays
per JSON-RPC and accepted 0044 §2, not a fabricated object wrapper.

## Behavioral red/green receipts — historical source-reconciliation run

Baseline commit `1c2cb35979172364af5c90363e4fad06e2a04f57`, tree
`62cf320bdd9960fe982f738dbc37d23a98986252`.
Tests-first patch SHA-256
`6da01b6e9db18c59c58b5aeb3d42bdab3c21cae5dedc401c6fe6d75647c823b8`
contains the first four regressions; their mcp_revisions bytes match the earlier
header/description/OpenAPI red runs. Run-local retained proof directory is
`.forge/issue152-reconciliation-proof/` (locator, not tracked hyperlink).
Authoring compilation errors were corrected and are not counted as red proofs.

| Repair | Exact command (CARGO_NET_OFFLINE=true) | Behavioral red, exit 101 | Same-test success |
| --- | --- | --- | --- |
| Non-POST headers | cargo test --locked --test mcp_revisions revision_transport_methods | header-red.log: GET excluded revision returned 405, expected 400 | revisions-green.log; all method/header shapes later extended |
| Modern wait wording | cargo test --locked --test mcp_revisions revision_tool_annotations | description-red.log: modern descriptor promises batching | revisions-green.log; legacy descriptors unchanged |
| Served OpenAPI | cargo test --locked --test mcp_revisions revision_served_contract | openapi-red.log: lacks 2025-03-26 only | revisions-green.log |
| Required initialize result fields | cargo test --locked --test stdio incomplete_initialize_results_cannot_enable_operations | initialize-red.log: next exchange initialized/null instead of ping 3; incomplete result installed state | stdio-green.log: 42 malformed results across three revisions; lost/truncated recovery separately added |
| Invocation rate | cargo test --locked --lib exhausted_tool_rate_refuses_before_effects_and_keeps_control_available | rate-red.log: seed isError false instead of true | rate-green.log; exact monotonic boundary and live transient observers |

Rate test used type/context/router scaffolding before enforcement. Final test was
rerun red with only the enforcement block absent, then restored and green.
Its baseline is the commit above plus rate-first.patch SHA-256
`18b41ba4f2911d964b8e2b2fc360c35134f76c4eaf3ef85e2a5cf86270311fcc`.
No expectation-only rewrite, environment failure or fabricated red counts.
Added witnesses for existing behavior need no artificial baseline failure.
Those historical successes are not current-candidate proof; current workspace tests and exact coverage rerun the existing witnesses without behavioral edits.

## Source byte manifest

```text
86c6929d37272f1ac946d547a76742d1de0665b9cf6b64b879eabad3e4a1fd0d  2025-03-26/spec/basic/index.mdx
271f4c9751583fa5d8afe3efb09da5d622707664cd2eb764b7ec5713fb161b15  2025-03-26/spec/basic/lifecycle.mdx
320118fe48117b83cec097bfa1923258826f5e21abde7929b9dff0014236dc57  2025-03-26/spec/basic/transports.mdx
0a36ed8c4506483fba6175ee3ecea8d9140b18b7b3d8b4355aca14e9227cadfb  2025-03-26/spec/server/tools.mdx
bdc457208cb1b49061f3fbbae24144b7dbae282c746d82aa689ddd98329743e7  2025-03-26/schema/schema.ts
e720669548c8100a4282c49e580efd6ddf7f28899ea786fc8db251dbdb356131  2025-03-26/schema/schema.json
c1dc3ce16b22a9bbcf330da334b3b6bf72312c81a95ec42b9bd94460efa86087  2025-06-18/spec/basic/index.mdx
9800e66e16cb0b71c339f6007fd14f6d17cc48741d4a8d381cc38f916bb6d05e  2025-06-18/spec/basic/lifecycle.mdx
df1217279334b6f3af8bb457191884ba831dce2c5389d7a0556ba920270ca902  2025-06-18/spec/basic/transports.mdx
6c0bc03b6579eef309a4a81660a707df20360d011e464c1b74eae56f96e6417e  2025-06-18/spec/server/tools.mdx
7d7f4cc6f3adb76e3fd5e1eee6bab8b1e183cfaf719da66d2aa5661737ca9539  2025-06-18/schema/schema.ts
af845e7e5b9d27107d1690f0936022546177a1403e63ffb11470135b296a2e01  2025-06-18/schema/schema.json
bd275064995d6e36dbb51c059be97e81c3eb7ceafc932e0276a7fc0a84c30fa4  2025-11-25/spec/basic/index.mdx
45a6e8b7fb8c96e7b9ba1b0a3c727e8451c1e55bf56bb62f3ab63fddc365b919  2025-11-25/spec/basic/lifecycle.mdx
a247fdbb3cc25c805ef43124db18d9b60a56669b3e65bd163dffb76f4129dfc0  2025-11-25/spec/basic/transports.mdx
39e56ad4f3d1ff1cb28ee62283e02947cd97db8aa6190782d629f4562a0f354c  2025-11-25/spec/server/tools.mdx
e74b56e73b2e37bdb595f74ba22e428ad7f07aa3519355ba661d681298ed38ac  2025-11-25/schema/schema.ts
268a5f82ba70fd7e4b6dc4aa1e64f116f74b4d0edcb69dc046829c79dd4e97e7  2025-11-25/schema/schema.json
```

## Delivery and outstanding ownership

Current candidate-bound local gate evidence is recorded in [implementation.md](implementation.md).
Prior design/source-reconciliation receipts are historical, not current implementation proofs. Exploratory coverage caught
the design's missing 0080 index row; repaired, never excluded from coverage.
Claude Code, Codex, MCP Inspector and Claude Desktop real-client receipts remain
**operator-owned and owed**. CI owns macOS/chart/history/Docker/RustSec/port 80;
operator owns live settings, decision acceptance, #151 baseline, sealing/merge.
No extra local network/namespace/platform criterion is invented. No push, merge,
tag, publication, paid trial, closure or acceptance occurred. Local reconciliation
does not discharge retained source deviations or external obligations by implication.
