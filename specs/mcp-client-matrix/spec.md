# Record the MCP client matrix

Part of #392, epic #164. An operator needs to know which installed clients can discover and use Swarmail's tools on a particular main build. A local driver proves server behavior. Only an observation through the named client can establish what that client received. This delivery supplies the procedure and checks the shape of the evidence. The four live runs belong to the operator. The compatibility criterion requires green recorded results for all four clients. [Negotiation ruling](../../docs/decisions/0079-mcp-version-negotiation.md) · [Source reconciliation](../../docs/decisions/0080-mcp-source-reconciliation.md)

## Acceptance Criteria

1. Both agent setup and the MCP guide link to one checklist at `docs/agents/client-matrix.md`. It gives four individually executable routes for Claude Code, Codex, MCP Inspector and Claude Desktop. Each links to the existing server and transport wiring and identifies the exact actions, tool arguments, expected replies and a way to observe that client's raw exchanges. It labels CLI headless and person-operated routes.
2. Each observation set uses one fresh memory-only loopback server from a clean main checkout, one installed client session and one synthetic multipart SMTP delivery. The procedure identifies the executable actually launched, its SHA-256 and full Git commit, confirms the final DATA acceptance and retains its stored message ID. It gives a copyable fixture and delivery command. The subject, verification code and tiny named attachment have known expected values.
3. Every client records these six independent outcomes in the same session. A pass requires the matching client request and successful response. A failure, unavailable observation or blocked step records a short specific reason.

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
