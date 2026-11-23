# MCP client receipts

One receipt per client and date, named `YYYY-MM-DD-<client>.json`, where
`<client>` is `claude-code`, `codex`, `mcp-inspector` or `claude-desktop`.
The checklist, the capture procedure and the field table are in
[`docs/agents/client-matrix.md`](../../agents/client-matrix.md). Part of
issue #392.

`cargo test --locked --test agent_setup_docs client_matrix` checks every
file here except this README. It checks the shape of each file only, and it
prints `no receipts yet` while there is none. A receipt is recorded by the
operator, or by a dispatcher on the operator's behalf, from an observation
through the named client. Synthetic receipts live only inside the tests'
temporary directories, never here.

**Status, 2026-11-22: no receipts yet.** None of the four clients has a
recorded result, so the compatibility exit criterion of epic #164 is not met.
