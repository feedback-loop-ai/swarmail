# Implement the client evidence checklist

## Technical context and research

The setup guide owns the working HTTP and stdio snippets. Its root integration target already checks their content. The negotiation implementation record calls for client/version, candidate, transport, revision, initialize/list/call results, location and verdict. The receipt maps those observations to a file and six step outcomes. March legitimately omits structured output. [Setup](../../docs/agents/setup.md) · [Implementation evidence](../mcp-version-negotiation/implementation.md) · [Structured output ruling](../../docs/decisions/0082-mcp-structured-output.md)

Adopt robustness's separate search call. Combining the read and structured-output verdicts as simplicity proposed saves a call, but loses whether the client could read mail when it received only text. Four operators' runs incur one extra local call each. Adopt robustness's tracked directory README so a path typo or failed directory read cannot masquerade as an empty matrix. Its cost is a short permanent file. Use simplicity's single typed reader and manual collection. CLI automation would need client authorization, handshake capture and process cleanup. A fake binary could test the launcher alone. Manual capture costs operator time and introduces transcription risk.

## Project structure and operator quickstart

- Add the shared fixture, steps, receipt field table and four client routes in `docs/agents/client-matrix.md`. Link to setup sections 1–3 for wiring. Give a literal delivery example using a synthetic MIME message, SMTP on loopback and the fresh server's default inbox. Take the ID from the final DATA reply. Give commands to identify main's clean commit, hash the executable at its actual launch path, and read the client version (About for Desktop).
- Start private observation before connection. For HTTP give concrete instructions to inspect the actual client's loopback request and reassembled response, matching connection and RPC ID. For stdio provide a temporary private pass-through capture of bridge input/output with stdout reserved for protocol bytes. Document how to inspect and discard the capture. A separate handshake or a model's prose cannot fill a missing observation.
- Claude Code opens the configured project, approves the HTTP server, checks its connection, then invokes the four calls in checklist order. Codex opens a configured HTTP session, inspects the MCP listing and requests the same calls, retaining setup's config-path caveat. Give copyable prompts with tool names and arguments, withholding the expected code. Identify Claude Code print mode and Codex exec as headless-capable on an authorized host.
- Inspector uses an installed Inspector's Streamable HTTP connection, Connect, Tools, List Tools and the argument/response panes for each call. Desktop uses Settings → Developer → Edit Config, the linked stdio entry with an absolute bridge path, a full restart, then connected tools in a conversation. Give the observation point for each route. Both documented UI routes require a person.
- Link the checklist from `docs/agents/setup.md` and `docs/guide/mcp.md`. Add location and dated initial status to the MCP guide, `specs/mcp-version-negotiation/implementation.md` (append to its historical evidence), and `docs/evals/README.md`. Track `docs/evals/clients/README.md` with the naming rule and checklist link. Keep all synthetic receipts inside temporary tests.
- Record delivery evidence, test cost and operator work in `specs/mcp-client-matrix/implementation.md`.

## Data model and validation

Implement one directory-taking validator in the existing `tests/agent_setup_docs.rs` docs target. Reuse its repository path conventions, serde/serde_json, chrono and tempfile. Deserialize directly from bytes into strict receipt, fixed-step and outcome types. Decoding via a generic JSON map first would collapse duplicate keys. Use a required value representation or explicit presence check for nullable evidence. Model pass and fail with their exact field sets. Check calendar round-trip, basename, hex lengths, enums and nonblank strings. Preserve protocol-owned JSON as a value. Compare the recorded revision to the observed result, including null when unavailable. A success requires a result and excludes an error. Refuse downstream passes when initialization produced no usable session.

Ignore only the directory README during enumeration. Require regular receipt files with the exact basename and propagate filesystem errors. Run the same function on real and temporary directories. Print the empty status after successful enumeration. Quickstart for the focused implementation proof is `cargo test --locked --test agent_setup_docs client_matrix -- --nocapture`; give the new tests that name prefix. A synthetic passing receipt, March partial failure, raw initialize error and unavailable reply cover the evidence states. Generate each invalid input independently from a valid temporary copy. Exercise duplicate names at the serialized byte level.

## Constitution check

This is an operator evidence format implementing accepted 0079/0080. It introduces no runtime or public MCP API change requiring a decision. The procedure uses stored mail after SMTP acceptance and ingest-time extraction. It requires no changes to storage, waits, pruning, production dependencies or held paths. Run the house gate inventory on implementation. Keep new tests portable file operations, with no subprocess, credentials, network, polls or waits. The operator performs live runs separately.

## Risks

| Risk | Mitigation |
|---|---|
| Cached discovery, a CLI summary or a parallel probe gives a false pass | Match raw exchanges from the actual session. If evidence is hidden or absent, fail the affected step and explain why. |
| Restart, stale mail or an unrelated delivery supplies a plausible result | Isolate each fresh instance, disable chaos, match the accepted ID throughout and abandon a disrupted session. |
| Raw evidence discloses a path, token, host or account | Use only synthetic mail. Retain captures privately, redact identifying values in place while preserving protocol fields and revision, review reasons, and save only the receipt. |
| An empty directory or a well-formed failure is mistaken for compatibility | State dated evidence status, keep failure reasons, and leave the four-client verdict to operator evidence. |
| File checking consumes CI budget | Use small temporary cases in one existing target, measure the focused tests on a warm build and report elapsed time separately from build time. |
