# Implement the Inspector receipt runner

## Technical context and panel rulings

Build on the landed #399 checklist, directory scanner and documentation tests. The dispatcher framing dated 2026-11-24 supplies the intake in this checkout. The work belongs in the standalone Rust evals workspace. The existing MCP endpoint, fixture values and typed receipt contract already cover the six observations.

Adopt simplicity's one Inspector route, shared reader and existing SMTP delivery helper. Reject a general client adapter registry. Supporting another command grammar would require a separate mapping from client-visible results to evidence. Keep Claude Code and Codex on the documented approval-gated routes and Desktop on its interactive route.

Rule for simplicity on one process per CLI method. Robustness's single-session rule would make a one-method CLI unable to earn a green receipt even when all five invocations worked. Accept the narrower evidence about continuity, label it in the handbook, and keep the one-session UI procedure. Adopt robustness's requirement to bind the handshake, actual method and client-consumed result to the same invocation. That costs a bounded observer and CLI output decoding. Exit status alone misses a successful process that printed a tool error. [Proposed evidence rule](../../docs/decisions/0140-inspector-cli-evidence.md)

## Project structure, data model and contracts

1. Extract the receipt types and validator from `tests/agent_setup_docs/client_matrix.rs` into `evals/src/client_matrix/receipt.rs`. Expose byte-plus-basename validation and the directory scan. Compile the same source into the root docs integration target by test-only path import. Keep guide/example and checked-in directory checks there. Move mutation coverage into evals. Preserve direct typed deserialization from original bytes, named-object checks, file-kind checks and diagnostics. Keep the twelve fields in the handbook, including required nullable handshake fields. Serialize drafts through the same contract. All non-null captured replies must satisfy its envelope rules. A malformed response that cannot be represented truthfully is a reported collection error.
2. Add `client-matrix run` and `client-matrix validate` through `evals/src/main.rs`, with orchestration in `evals/src/client_matrix/` registered from `lib.rs`. Accept absolute candidate binary, clean-main commit, exact installed Inspector package version, OS/version, recorder, draft directory and trailing client argv. Set observation date from UTC, hash the candidate with `identity::file_hash`, and require the operator to identify that binary from the documented clean build. An arbitrary working directory HEAD cannot establish provenance. Compute the required date/client basename under the resolved draft root.
3. Reuse the owned child and readiness path behind `runner::start_swarmail`, the bounded process supervision in evals and `mail::send_stored` with unauthenticated SMTP. Use a clean environment, private scratch directory, memory storage, default inbox, disabled chaos/hooks and dynamically assigned loopback ports. Use the checklist MIME fixture as the shared source (extract the fenced message or link one fixture from both consumers). Capture the final DATA ID once. Abandon setup after uncertain delivery or lost server ownership. REST seed accepts text/HTML and cannot supply the named attachment.
4. Extend the HTTP-only forwarding portion of `evals/src/capture.rs` with a bounded run-local observer where request and response bytes are already paired. Give each invocation a distinct loopback endpoint or equivalent lifetime isolation. Seal its observations before advancing. Preserve upstream status, protocol headers, notification handling and the existing Host/Origin translation. Route only the launched client's MCP traffic here. Keep transient data in bounded memory. Retain the first raw JSON-RPC initialize reply with extension fields and the handbook's in-place redactions. Persist only that reply, identities, outcomes and short sanitized reasons.
5. Pass the observed endpoint and Inspector flags as argv. Use `--cli <url> --transport http --method tools/list` first, then `--method tools/call --tool-name <name>` and repeated `--tool-arg key=value` per handbook arguments. Require one unambiguous handshake and expected exchange per launch, matching JSON-RPC IDs within each pair and the agreed revision for protocol traffic. Allow lifecycle notifications. Check HTTP, envelope, tool error state, returned values and client output/exit. For read, wait and attachments, decode the tool's documented text payload where applicable. For search require the structured object itself on the wire and in the consumed result. Preserve observed initialization independently of method success. Reused IDs across invocations are ordinary. Give a blocked or missing method a reason. Never retry to turn a red into a green.
6. Use one monotonic run budget with bounded per-stage waits, byte caps and bounded termination/reaping on success, error, timeout and interruption, including child process groups. Resolve scratch/output roots through `artifacts::resolve_root` before `safe_root`. Stage privately on the destination filesystem, validate serialized bytes against the final basename and atomically persist without clobber. Print the draft path and outcome. Return success for all-pass, a distinct nonzero status for a written red draft, and a command error for setup, invalid evidence or persistence failure.
7. Update `docs/agents/client-matrix.md` and `docs/evals/clients/README.md` with command usage, evidence boundaries and the validation entry point. Reuse setup guide wiring and version instructions. Check the existing status links in `docs/guide/mcp.md` and `specs/mcp-version-negotiation/implementation.md`. Keep the dated evidence status and operator obligation. Record timings and proof in `specs/mcp-client-matrix/implementation.md`.

## Operator quickstart after implementation

Build the identified executable using the handbook's clean-main procedure. Use the OS version and the installed Inspector package version from that same host and installation. With those values set, the evals workspace invocation is:

```sh
cargo run --locked --bin swarmail-evals -- client-matrix run \
  --binary "$BIN" --commit "$MAIN_COMMIT" --client-version "$INSPECTOR_VERSION" \
  --os "$OS_VERSION" --recorded-by dispatcher --output-dir "$DRAFTS" \
  -- npx --offline --no-install "@modelcontextprotocol/inspector@$INSPECTOR_VERSION"
cargo run --locked --bin swarmail-evals -- client-matrix validate "$RECEIPT"
```

Here `RECEIPT` is the draft path reported by the runner. An installed executable path is also a valid prefix. Set npm offline and disable installation for the npx route, require the package to be present, and execute argv directly. Keep this route marked pending a real host Inspector observation. The implementation proof supplies a local test executable through the same interface.

## Constitution check

One private delivery per server makes final DATA acknowledgment sufficient readiness under 0001. The zero-timeout wait exercises subscribe-before-scan under 0002. Oldest-only pruning and ingest extraction continue to govern the fixture under 0003, 0005 and 0110. March text-only search fails the structured step under 0082. Keep server code, production dependency closure, locks, held paths and gate inventory intact. 0079/0080 continue to require operator evidence for all four clients.

## Risks

| Failure | Mitigation and proof |
|---|---|
| CLI hides a handshake, discards a result, reconnects or reuses an ID | Join evidence within each process lifetime. Test a successful exit with an absent exchange, mixed revisions and a client output mismatch. |
| A stale process or a second delivery supplies the expected OTP | Check owned startup, keep one server and accepted ID, match every result to that ID, and stop on a restart. Exercise the full chain against a real candidate. |
| Capture or client output leaks machine data or fills memory | Cap both streams before parsing, redact only identifying values, sanitize reasons and discard traffic at shutdown. Test limits and retention with synthetic input. |
| Timeout, interrupted output or an existing draft produces misleading evidence | Bound and reap the process tree, validate before atomic no-clobber install, and exercise collision and cleanup paths. |
| Linux ownership checks break macOS tests or long waits exceed the CI budget | Put Linux cfg directly on each Linux-only helper. Apply the required non-Linux ignore to every binary-launching or proc-reading test. Keep pure tests portable, use resolved temp roots and immediate fixture outcomes. Measure incremental warm runtime separately from compilation. |
