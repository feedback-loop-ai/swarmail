# Record the MCP client matrix

Part of #392, epic #164. An operator needs to know which installed clients can discover and use Swarmail's tools on a particular main build. A local driver proves server behavior. Only an observation through the named client can establish what that client received. This delivery adds an offline Inspector CLI runner and shares the receipt checker with the standalone eval harness. The four live runs belong to the operator. The compatibility criterion requires green recorded results for all four clients. [Negotiation ruling](../../docs/decisions/0079-mcp-version-negotiation.md) · [Source reconciliation](../../docs/decisions/0080-mcp-source-reconciliation.md)

Decision: 0138

The proposed Inspector CLI evidence rule specifies what a headless Inspector receipt attests. [Proposed decision 0138](../../docs/decisions/0138-inspector-cli-evidence.md)

## Acceptance Criteria

1. Both agent setup and the MCP guide link to one checklist at `docs/agents/client-matrix.md`. It gives four individually executable routes for Claude Code, Codex, MCP Inspector and Claude Desktop. Each links to the existing server and transport wiring and identifies the exact actions, tool arguments, expected replies and a way to observe that client's raw exchanges. It labels CLI headless and person-operated routes.
2. Each observation set uses one fresh memory-only loopback server from a clean main checkout, one installed client build and one synthetic multipart SMTP delivery. The procedure identifies the executable actually launched, its SHA-256 and full Git commit, confirms the final DATA acceptance and retains its stored message ID. It gives a copyable fixture and delivery command. The subject, verification code and tiny named attachment have known expected values.
3. Interactive and existing agent routes record these six outcomes in one session. The Inspector CLI route uses the invocation boundaries in criterion 8. A pass requires the matching client request and successful response. A failure, unavailable observation or blocked step records a short specific reason.

   | Step | Evidence required for pass |
   |---|---|
   | `initialize` | Capture the actual JSON-RPC initialize reply and read its returned protocol revision. |
   | `list_tools` | Observe tools/list from the client and find all four tools named below in the returned descriptors. |
   | `read` | Call swarmail_get_latest_email with the fixture inbox and count 1. Match the stored ID and subject. |
   | `wait_for_code` | Call swarmail_wait_for_code with that inbox and timeout_ms 0 after delivery. Verify selected status, the expected code and its email_id. |
   | `structured_content` | Call swarmail_search_emails for that inbox. Inspect result.structuredContent as an object with the fixture ID in emails. A text block alone fails this step, including under March negotiation. |
   | `list_attachments` | Call swarmail_list_attachments with the stored ID. Match that ID and the fixture's attachment filename, content type and decoded size. |

4. One UTF-8 JSON receipt at `docs/evals/clients/YYYY-MM-DD-<client>.json` records the exact fields below. The filename is the receipt location. Preserve an existing client/date observation. The operator reviews the redacted exchange before recording. Status is per step, so a receipt can accurately describe a failed run.

   | Required field | Value |
   |---|---|
   | `date`, `client` | Real calendar date in canonical YYYY-MM-DD form and one of claude-code, codex, mcp-inspector, claude-desktop. Both match the whole basename. |
   | `client_version`, `os` | Nonblank strings giving the exact installed client version/build and OS. |
   | `swarmail_commit`, `executable_sha256` | Full Git object ID (40 ASCII hex digits in this repository) and executable digest (64 ASCII hex digits). |
   | `transport` | http or stdio, as used by the client. |
   | `negotiated_revision`, `initialize_response` | Returned protocolVersion string when available, otherwise null. Preserve the complete observed JSON-RPC response object after redaction, or explicit null when no JSON response was captured. |
   | `steps` | Exactly the six keys above. Each object has status pass or fail. A fail also requires a nonblank reason. A pass has only status. |
   | `recorded_by`, `mode` | operator or dispatcher, and interactive or headless, respectively. |

5. The file-only check validates every receipt, including failed observations. It rejects missing, extra or duplicate receipt-owned fields, unknown or missing steps, invalid types, enums, dates and hashes, filename disagreement and inconsistent handshake evidence. A passing initialize requires a JSON-RPC 2.0 success response with ID and a matching nonblank result.protocolVersion. Preserve a captured error response on failure. If initialization blocked later observations, record each as failed. Protocol extension members within the captured response survive. Absence of a required nullable key is an error.
6. A successfully enumerated empty receipt directory passes and emits exactly “no receipts yet” as its status message. Enumeration, read and parse failures surface as errors. Diagnostics identify the offending path and field or failed rule. Temporary-copy tests remove every required field individually, including nested keys and conditional reasons. The validator reports file validity only.
7. The MCP guide and negotiation implementation record link to the receipt location and state the dated zero-receipt status. The eval README carries the dated status line. The implementation ledger says “Part of #392”, records the measured added test time and identifies the outstanding operator runs. No compatibility verdict follows from CI, a fake client, or an empty directory.

8. The evals runner accepts an installed Inspector command as literal argv. It drives five sequential Streamable HTTP method invocations against one owned server and fixture. Each invocation must initialize successfully at the first negotiated revision, complete the expected exchange and return the expected data to the CLI. Scope request/reply ID matching to that invocation. The first invocation supplies the receipt's initialize evidence, even if discovery subsequently fails. A missing, ambiguous, failed or different later handshake fails its method. An unusable first handshake blocks all remaining steps. A headless HTTP Inspector receipt made by this procedure attests per-method operation across fresh sessions. Interactive Inspector still proves the single-session procedure.
9. The runner starts the given binary memory-only on ephemeral loopback ports, hashes the launched file, and takes its clean-main build identity from the operator's build record. It delivers the handbook fixture over SMTP and uses only the ID in the final DATA acceptance. The client performs every scored request. A setup failure before an attributable observation returns a command error. An observed client failure yields a validated red draft identifying the step. Independent later calls still run while the server and budget permit. The runner bounds time, data and child lifetime, reaps descendants, and installs one complete validated draft without overwriting or publishing a file.
10. A single evals-owned validator enforces criteria 4–6 for drafts, the documentation example and the recorded directory. Portable cases independently omit and mistype each required field, including nested outcomes and conditional failure reasons. The evals runner's public command produces a six-pass synthetic receipt and a valid red receipt against the candidate Swarmail binary using a local stand-in speaking real MCP. A successful client exit with missing wire evidence cannot earn a pass.
11. The checklist gives the operator the installed/offline Inspector invocation and version source, preserves the UI route and links the other clients' existing configuration. Paid routes require operator approval and Desktop requires a person. The guide and negotiation implementation record retain “owed” and link to the procedure and format. This delivery commits zero receipts. The implementation ledger records the incremental CI test cost, the commands measured and the remaining host Inspector verification. All implementation gates in the house inventory apply.
