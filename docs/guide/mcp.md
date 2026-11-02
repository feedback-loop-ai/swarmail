# MCP — agents drive the mail

[← README](../../README.md) · [Guide index](README.md)

MCP negotiates `2025-03-26`, `2025-06-18` and `2025-11-25`. A single
`initialize` returns a supported requested revision; an unsupported string
receives `2025-11-25` so the client can decide whether to continue. Omission
keeps `2025-03-26`; a supplied non-string version refuses `-32602`.
Legacy batch initialize always returns `2025-03-26`.

HTTP clients send `Content-Type: application/json`,
`Accept: application/json, text/event-stream` and, after negotiation,
`MCP-Protocol-Version: <negotiated revision>`. The JSON-only endpoint tolerates
omitted media headers. It is stateless: a missing version header selects
`2025-03-26`; exactly one supported header selects that revision, with no
initialization-history enforcement or session IDs. Invalid, duplicate or
unsupported version headers refuse HTTP 400 / `-32600` / null before effects,
after POST body-limit and parse checks. Invalid headers also refuse on other
methods; otherwise GET returns 405. SSE is not implemented.

**Batches are available only under `2025-03-26`.** All batch descriptions
below apply to that revision. Either modern revision refuses every array with
one `-32600` / null error naming the revision, before any member executes,
including arrays containing only notifications. Modern tool descriptors add
`readOnlyHint: true` on reads, and `readOnlyHint: false, destructiveHint: true`
on all mutations (including seed, which can evict old mail, and chaos changes).
Input schemas, the thirteen existing March catalogue entries and existing
result bytes are unchanged for March (the modern revisions add `fields` to
four reads, below); discovery appends the two focused waits and
swarmail_create_inbox after them (decisions 0094 and 0112, proposed), sixteen
entries in all, then the two attachment tools (decision 0110, proposed),
eighteen in all. For `2025-06-18` and
`2025-11-25`, every tool declares an object-root `outputSchema`; successful
results include `structuredContent` and the same JSON in the existing text block.
Errors retain their existing text/RPC envelopes without structured success data.
The REST-backed tools' schemas are pinned to served OpenAPI through explicit
output projections in tests; the two focused waits and the two attachment tools
are MCP-only, with no REST projection, and their schemas are checked against
independently stated expectations.

**Compact reads and `fields` (decision 0108, proposed).** On `2025-06-18` and
`2025-11-25`, `swarmail_search_emails` and `swarmail_get_latest_email` answer a
compact view of each email by default: `id` (the handle — pass it to
`swarmail_get_email` for the whole email), `from`, `to`, `subject`,
`received_ms`, a `snippet` of at most 200 characters (the text part, else the
HTML rendered to text, from at most the first 8192 bytes of the body),
`codes`, `anchors` (`url` and `anchor_text`) and `attachments` (`filename`,
`content_type`, decoded `size`). `anchors` and `attachments` show at most 250
entries and carry the exact `anchors_total` / `attachments_total` when cut.
Mail stored before link occurrences were kept serves its stored `links` instead
of anchors, and mail stored before attachments were captured, or whose body the parser
could not read, serves `"attachments": null` (unknown, never guessed from the
raw bytes). Attachments
are listed at ingest, in the same transaction as the mail. `swarmail_get_email`
and `swarmail_wait_for_email` keep answering whole emails. The four reads
(search, latest, get, wait) also take `fields`, an array of names from the
published enum (the serialized email members plus `snippet`, `anchors` and
`attachments`): each email then carries those members that it has, plus `id`,
in the server's fixed key order — `fields: []` is ids only, and `text` and
`html` appear only when named. An unknown name or a wrong type is `-32602`
before any lookup or wait. March (`2025-03-26`, and a missing version) keeps
whole emails and has no `fields`. REST is unchanged unless asked:
`GET /api/v1/inboxes/{inbox}/messages?view=compact` answers the same
`{total, emails}` envelope with compact items; `compact` is the only value of
`view`, and no other route takes it. The token metric (median tokens per
success) is not measured by this change: no paid trial was run.

Duplicating JSON increases response size and token cost. The stdio bridge keeps
its 10 MiB response cap: a query that fits as March text may exceed it with modern
dual output and return `-32000`. Use smaller pages; a single large email can still
exceed the cap. Lost replies can follow committed mutations; do not replay them
assuming rollback. The epic measures tokens-per-success and invalid-argument/
tool-misuse rates; local tests prove representation and text compatibility, while
the operator's #151 baseline and client matrix own measurement and client receipts.

The stdio bridge starts with legacy compatibility. Single initialize fences
later input until a matching successful response with capabilities and server
name/version selects the revision; it omits
the version header for initialize and sends the returned version afterward.
Modern clients then send `notifications/initialized`; operations wait for its
empty HTTP 202 acknowledgment. Ping remains available. Early operations and
repeated modern initialize refuse `-32600`; notifications stay silent without
effects. Failed modern negotiation requires an explicit initialize retry and
cannot release queued effects under legacy rules. Modern arrays refuse locally
without a connection, including oversized arrays (one null-ID size refusal).
Operational lines remain concurrent after the handshake.

Tool invocations share a per-server budget of 1024 attempts per one-second
monotonic window, including notifications and March batch members. Exhaustion
returns a text `isError` result (notifications remain silent) before effects;
initialize, ping and discovery remain available. This is independent of active
wait counts and does not rate limit SMTP or REST.

The bridge targets this stateless JSON server and parses identity HTTP bodies.
It advertises the mandated JSON/SSE Accept tokens but does not implement an SSE
client, sessions or general Streamable HTTP client conformance. These limits,
unsolicited-response disposition, ID tolerance, legacy sender omissions, error
taxonomy and malformed peer envelopes on stdout are documented deviations C1–C6,
retained by the operator's 2026-10-26 choice. This delivery claims version negotiation; full
revision conformance is not claimed. The [deviation register](../../specs/mcp-version-negotiation/reconciliation.md#exact-conflict-register)
records each revision's behavior and compatibility cost. Email
content is untrusted data: tools return text containing JSON string fields,
never active HTML, executable resources or fetched links. Consumers must retain
that data boundary when displaying results or passing mail to a model.

The metric is compatibility: actual Claude Code, Codex, MCP Inspector and
Claude Desktop receipts remain operator-owned and owed (issue AC3). See the
[implementation evidence](../../specs/mcp-version-negotiation/implementation.md).
The operator holds this PR unmerged until #151's baseline exists.

```
$ curl -s -X POST localhost:8025/mcp -H 'content-type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{…}}'
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-03-26",
  "serverInfo":{"name":"swarmail","version":"0.1.0"}, …}}

$ curl -s -X POST localhost:8025/mcp -H 'content-type: application/json' \
    -d '{"jsonrpc":"2.0","id":2,"method":"tools/call",
         "params":{"name":"swarmail_wait_for_email",
                   "arguments":{"inbox":"quickstart","to":"ada@x.io","count":1,"timeout_ms":2000}}}'
{"jsonrpc":"2.0","id":2,"result":{"isError":false,
  "content":[{"type":"text","text":"{\"emails\":[{…\"codes\":[\"424242\"]…}]}"}]}}
```

Eighteen tools: `swarmail_search_emails`, `swarmail_get_email`,
`swarmail_get_latest_email`, `swarmail_delete_email`, `swarmail_clear_inbox`,
`swarmail_clear_all`, **`swarmail_wait_for_email`** (blocks until arrival),
`swarmail_extract_links`, `swarmail_extract_codes`, `swarmail_seed_email`,
`swarmail_set_chaos`, `swarmail_clear_chaos`, `swarmail_get_deliverability_report`,
the two focused waits appended after them, **`swarmail_wait_for_code`** and
**`swarmail_wait_for_link`** ([focused waits](focused-waits.md)), then **`swarmail_create_inbox`**
(mints a private inbox and its routed address; needs `--route-domain`), and
last the two attachment tools, **`swarmail_list_attachments`** and
**`swarmail_get_attachment`** ([attachments](attachments.md)). Except for malformed seed fixture
content and the supplied
`timeout_ms` refusal described below, every tool validates its supplied
arguments at the envelope: a wrong-typed or malformed value — the
`arguments` container included — returns JSON-RPC `-32602` in HTTP 200
for a call with an `id`, naming the field and its domain, before any
effect, and so does an argument key the tool does not publish
(`{"inbx": "zz"}` can never clear `default`; every schema says
`additionalProperties: false`); an unknown tool name is `-32601`; a JSON
parse failure or invalid UTF-8 is HTTP 400 `-32700`; a body over the
2,097,152-byte limit is HTTP 413 `-32600`; a value that is not an object
(`5`, `null`, `"x"`, a nested array) is `-32600` with a null id — once per
such member in a batch; an object without an `id` is a notification only
when its `method` is a string: it runs and stays silent, whatever the
method names. Any other `id`-less object — `{}`, `{"foo":"boo"}`,
`{"method":1}` — is `-32600` with a null id, alone or as a batch member
(decision [0047](../decisions/0047-storage-rpc.md), issue #132).
`swarmail_search_emails` pages with `offset` past
its 250 limit. The members of one batch (at most 16) run concurrently:
sixteen waits share one wait interval rather than queueing, replies keep
member order, and the members' effects have **no** order — send dependent
mutations as separate requests.

An elapsed `swarmail_wait_for_email` timeout is a normal tool result:
`isError: false`, with `matched`, `total` and `timed_out: true` in its text
content. On `swarmail_wait_for_email`, a supplied `timeout_ms` must be an
integer from 0 through 60000; malformed values and values above 60000
return an HTTP-200 tool result with `isError: true` and refusal text, not
a JSON-RPC `error` object.
`swarmail_seed_email` refuses `CR/LF/NUL` in its `to`, `from` or
`subject` fixture strings before insertion under decision
[0010 §2](../decisions/0010-exact-count-asserts-and-the-shared-seed.md);
for a call with an `id`, it returns HTTP 200 with a tool result
`isError: true` whose text names the offending field and its reason;
no message is inserted. Wrong-typed seed arguments and unknown keys
remain JSON-RPC `-32602`; id-less notifications stay silent.

Tool failures such as `shutting down` return
`isError: true` with an error message in text content.
An unknown MCP argument key returns JSON-RPC `-32602` in HTTP 200 for a call
with an `id`, before any effect. Other protocol and
argument refusals use a JSON-RPC `error` envelope as described above.

## Wire it up: agent setup and the stdio bridge

Wire it up: copy-paste setup for Claude Code (`.mcp.json` and a skill),
Codex, Cursor and dsh, and an `AGENTS.md` block that teaches the clear → act →
wait → extract loop, are in [the agent setup guide](../agents/setup.md).
The wiring snippets in `docs/agents/setup.md` are written into a fresh project
and driven against the real binary by `tests/agent_setup_docs.rs` in CI;
install commands are marked not-executed.

`swarmail mcp` speaks JSON-RPC over stdin/stdout and bridges to
`SWARMAIL_URL` (default `http://127.0.0.1:8025`). The HTTP surface is
Streamable JSON-RPC at `POST /mcp`. A URL without a port dials HTTP's port
80 and sends its `Host` exactly as spelled; a malformed port is refused
(decision [0047](../decisions/0047-storage-rpc.md), issue #133).

The bridge answers each line as `POST /mcp` would (decision
[0039](../decisions/0039-stdio-concurrency.md), updated by
[0044](../decisions/0044-compat-validation.md)). A batch with requests
gets one array line; a notification, or a batch of notifications only,
gets nothing; a line that is valid JSON but not an object — or an `id`-less
object without a string `method` — is an invalid request and gets the
server's null-id `-32600`. A line whose body is over the server's
2,097,152-byte limit (counted in bytes, without its line ending) is never
sent: the bridge answers `-32600` in the server's own 413 wording with the
line's own ids — one error, or one array in member order for a batch —
and nothing for a notification. Up to 16 lines are in flight at once, so a wait never holds
up the seed that satisfies it. Replies arrive as they complete, so order
holds only within a batch. A client that needs two mutations in order
should await the first reply. A line that is not UTF-8 or not JSON gets a
local `-32700` with a null id and never reaches the server. If the server
cannot be reached, each request id — and each non-object member, as a
null id — gets a `-32000` error, and a rejected batch gets one null-id
error. The outcome of such a request is unknown,
and nothing is retried. At EOF the bridge waits for every request in
flight to finish, then exits 0. It exits 1 if stdin fails (after waiting)
or if stdout fails (immediately). Known gaps: no length cap on reading or
parsing a single input line (the body limit bounds what is sent, not what
is held); a stalled stdout reader stalls the bridge; sixteen parked
waits delay a seventeenth line; a server or proxy with a lower body limit
than the bridge's still answers its own null-id refusal.
