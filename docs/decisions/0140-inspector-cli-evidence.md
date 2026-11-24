# 0140 — Record Inspector CLI observations across method invocations

- **Status:** proposed
- **Date:** 2026-11-24
- **Scope:** Part of #392. Evidence collected with a locally installed MCP Inspector CLI. [Client matrix specification](../../specs/mcp-client-matrix/spec.md)

The existing checklist assumes the client stays connected through discovery and four calls. Inspector's CLI takes a method per launch. Requiring connection continuity would make the scripted route depend on a feature the dispatcher cannot assume. Treating the five exit codes as observations would lose both negotiated revisions and tool errors.

Permit a headless Inspector set to span five process-local sessions on one fresh server, with one installed Inspector build and one acknowledged delivery. Capture the first initialize response and revision for the receipt. Bind every method to its own client handshake at that revision, its request and reply, and the result the client consumed. Record a failed step when that chain breaks. The existing client, mode and transport fields identify the documented route. Describe this meaning beside the receipt table. Interactive observations keep their single-session rule.

The cost is five handshakes and narrower evidence about session continuity. A reader who needs evidence of sustained interaction uses the Inspector UI route. Only real named-client observations count toward the four-client compatibility criterion. The operator owns acceptance and the actual receipts. [0079](0079-mcp-version-negotiation.md) · [0080](0080-mcp-source-reconciliation.md)
