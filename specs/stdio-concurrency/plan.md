# Implementation plan: concurrent stdio batches

Implements [spec.md](spec.md) and proposed
[0039](../../docs/decisions/0039-stdio-concurrency.md). This is a design handoff,
not evidence of implemented or passing behavior.

## Technical context

One Rust 2024 crate, Tokio I/O, serde_json and existing futures-util with
`std`. `src/stdio.rs` currently takes `Lines<R>`, parses a top-level ID, awaits
one `rpc_post`, and discards stdout errors. `rpc_post` already has injected
exchange deadlines, bounded response reads and a separate bounded dial.
`mcp::dispatch` handles arrays sequentially with `MCP_BATCH_CAP = 16`.
`tests/binary.rs` owns real subprocess helpers and ephemeral listening ports.
No new dependencies, features, production modules or transport library.
No plan template exists in `.specify/templates/`; use the house-required
technical context, constitution, structure, research, model, contracts,
quickstart and risks sections here. The available spec template describes
another workspace and does not override this repository's requested paths.

## Constitution check

0001–0003 and 0005 are preserved: server execution and store ordering stay
unchanged. 0039 is proposed because the stdio public contract changes.
0029 §2 remains the HTTP envelope authority. Tests follow accepted 0020:
branch-stable inline tests by default; real wire tests under `tests/`; only a
concrete cold failure path justifies an owner-adjacent cfg(test) harness.
Seams are instance-local, test-only and exercised. No coverage exclusions,
feature-flagged seams, gate edits, dependency/allowlist changes, sleeps as
synchronization, elapsed-time validity assertions or global mutable test gates.
Preserve exact 100/100/100 coverage under 0035 and all run-16 preservation rules.

## Project structure and files touched

| Path | Implementation work |
|---|---|
| `src/stdio.rs` | Byte reader, shared classification use, error shaping, 16-exchange futures loop, one checked writer path, lifecycle handling and inline tests. Update module contract. |
| `src/mcp.rs` | Small crate-private envelope classifier used by dispatch and bridge; preserve tool execution, cap and all HTTP responses. Inline table tests. |
| `tests/stdio.rs` | Adapt reader helpers; concurrent output draining; framing, transport, EOF and admission regressions; retain existing transport tests. |
| `tests/binary.rs` | Real mcp/serve batch parity, wait/seed/ping, malformed bytes and EOF regressions using existing cleanup helpers. |
| `tests/mcp.rs` | Pin classification parity against actual HTTP and notification effects. |
| `src/stdio/concurrency_tests.rs` (conditional) | Only if a deterministic private admission/reader seam needs cold timeout cleanup; document the concrete justification at its cfg(test) declaration. |
| `README.md`, `docs/decisions/README.md` | Document line concurrency, batch/error policy, bounds and residuals; index 0039 as proposed during implementation. |
| `specs/stdio-concurrency/{spec,plan,tasks}.md`, proposed 0039 | Exactly four design artifacts for this seat; later dated clarifications and evidence belong in their owning artifacts. |

Accepted decision text, Cargo manifests/lock, storage, SMTP/POP3, listeners,
gate scripts and bundles require no changes. The design commit must not
include run-local positions, intake evidence, result files or gate logs.

## Research and resolved choices

Read both panel positions, run-16 feature and issue bodies, house rules,
decision index, founding decisions, 0020, 0029 and 0035. No run-16 intake
exists in `.brokkr/state/tasks/`. Current code confirms both reproduced causes:
top-level-only `request_id`, and awaiting `rpc_post` before the next read.
The input cap, panic isolation and invalid-batch failure conflicts are ruled
explicitly in the spec; implementation must not silently resurrect them.

## Data model and algorithm

### Classification owned by MCP

Introduce a small crate-private classification (or equivalent enum) beside
dispatch: `Silent`, `Single(id)`, `Batch(ids)` and `InvalidBatch`. Classify a
parsed Value, checking empty/over-cap before collecting IDs; collection is
bounded by `MCP_BATCH_CAP` for admitted batches. Preserve IDs as JSON values,
including duplicates and null. An array member contributes only its own `id`,
never IDs nested in another array. Single id-less values are silent.

Dispatch uses the shared classification for envelope refusal and reply/silence
selection, while executing every admitted entry through existing `handle`.
Do not early-return `Silent` before executing notification tool calls. Keep
existing refusal messages, status codes, error codes and sequential per-batch
processing. Do not make `handle` recursively treat nested arrays as batches.
The bridge uses the same classification for success and -32000 shaping,
and calls `mcp::error_object` for local errors. Malformed input is a byte/JSON
parse error before classification. No second JSON parse for ID extraction.

### Byte framing and concurrent loop

Change `bridge` to take a mutable `AsyncBufRead + Unpin` instead of `Lines<R>`;
update its few callers directly. Keep a `Vec<u8>` partial-line buffer outside
the `select!` iterations. Use cancellation-safe `read_until(b'\n', ...)` so
a completion winning selection cannot discard bytes already consumed.
On complete line, decode UTF-8 then skip valid whitespace-only lines; otherwise
parse once and classify. A final nonempty buffer at EOF is processed once.
I/O-error partial fragments are not invented into complete requests.

Each valid parsed line creates one exchange future holding its input and
classification; no spawned task. Maintain `FuturesUnordered` with at most
`BRIDGE_CONCURRENCY = 16` entries. Its nonempty completion branch stays enabled
even with stdin idle. Enable the input branch only below capacity and while
input is open. At capacity consume no additional lines, including malformed
lines; resume after a completion. Do not allow empty-stream `next()` to spin.
Give ready completions precedence over further admission so an always-ready
stdin cannot starve existing exchanges; this does not prioritize tool methods.

Each completion returns an optional response value/text. `Outcome::Empty`
and `Silent` produce none. Batch errors use the spec table. One shared output
path compacts JSON using the already parsed HTTP value where practical, adds
newline, performs one `write_all` and flush, checking both errors. Preserve
response values and status/body validation, not insignificant whitespace.
No separate output queue. Local parse errors use the same writer path.

Outstanding state is at most 16 exchanges plus one partial input buffer and
one response being written. Each exchange retains input/IDs and may allocate
the capped response plus parsed/serialized forms; memory is not simply
16 times 10 MiB, because input, JSON overhead and copies remain. Avoid retaining
the full input Value after extracting needed metadata. Releasing a collection
entry before its write does not admit another request until the write returns.

EOF disables reads and drains the collection. Remember a fatal stdin error
as exit status 1 and drain admitted work; EOF otherwise ends with 0. Any
stdout failure returns 1 immediately, dropping the collection and sockets.
Retain deadline parameters for deterministic expiry tests. Do not add a global
drain timer: blocked stdout can stall indefinitely, as the spec states.

## Contracts and proof design

Real subprocess tests run `CARGO_BIN_EXE_swarmail mcp` against a freshly
spawned `serve` on ephemeral ports. Read stdout concurrently with input and
drain stderr so pipe buffering cannot deadlock the harness. Use per-test
inboxes and existing child guards; collect exact line counts at EOF, without
filtering blank lines. Compare parsed JSON values across transports, correlate
independent replies by ID, and assert order only where it is the contract.

For #72 cover two pings, mixed notifications, all notifications, request error,
empty/17-element batches. For #90 send wait/seed/ping adjacent and assert mail
with `timed_out: false`; cite 0002. Separately poll the existing labeled awaits
metric until the five-second waiter registers, send ping and require ping's
reply before releasing the wait with seed. Fail if a timeout reply wins; do
not hide it with retries or assert elapsed milliseconds. The server deadline
and existing 30-second helper bound detect failure, not synchronize the test.

For admission, arrange 16 held exchanges and observe a reader's poll/consume
boundary with an instance-local test seam or recording AsyncBufRead. Prove
the seventeenth line stays unread until a held exchange completes, then is
consumed; include a held notification exchange. For partial-line preservation,
pause input after a prefix, complete an earlier exchange, then release the
suffix. Use rendezvous acknowledgments, never sleeps or stress probabilities.
These structural seam tests complement, never replace, real-server acceptance.

Test write and flush errors separately with AsyncWrite fixtures, including an
outstanding held exchange that must be dropped before normal completion.
Test stdin I/O error after at least one admission, retaining that admission's
reply. Test malformed inputs with no reachable upstream and assert -32700;
also observe zero exchange admissions/POSTs through an exercised local seam
or real server observation. Raw transport parser fixtures stay local helper
tests; product behavior is proven against real serve, not a mock tool server.

For shared classification, table-test single null ID, id-less/methodless
objects, scalar values, nested arrays, duplicates, mixed entries, empty arrays,
and both kinds of oversized array. HTTP parity tests must also prove a silent
notification still executes its effect. Pin local failures across each
classification; preserve existing dial, response, long-wait and URL tests.

Capture at least one real-binary behavioral red per child against reviewed
`f39813d00162a6f87c675a90e6bd0e49f6925b32` or the unchanged bridge at this
run's base. Use an isolated baseline checkout when authorized by filesystem
permissions; never reset the working tree. Record test command, baseline SHA,
failed assertion and repaired SHA/result in ignored run-local evidence.
A compile failure or missing new helper is not a red; transplant independent
binary tests or label an old-behavior mutant and preserve its observation seams.

## Quickstart and validation

Implement the ordered [tasks](tasks.md) on a feature branch, running targeted
tests as each change lands. Before implementation delivery run:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test --workspace --locked --test stdio --test binary --test mcp
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh
cargo deny check licenses
```

Use no TMPDIR override. Coverage is exact in lines, functions and branches.
Run the CI-compatible formatter rather than hand-formatting. Report any
environmental refusal accurately; it is not a passing gate. Implementation
owns README/index updates and the later protected review/PR/17-check landing
workflow. This design seat does not push, merge or close issues.

For design validation, check all four files are nonempty, headings and
checkboxes exist, relative decision links resolve, and 0039 says proposed.
Stage exactly those four files and use a signed imperative commit with a
rationale body. If Git metadata is read-only, retain the reviewable artifacts
and report the missing commit explicitly; never redirect Git metadata to
evade the boundary or claim committed paths that are merely present.

## Risks

| Risk | Mitigation / accepted cost |
|---|---|
| Shared classification suppresses notification side effects. | Dispatch executes notifications before choosing silence; HTTP effect and parity tests. |
| Cancelled read loses a partial line or an empty future set spins. | Buffer survives select iterations; guarded completion branch; deterministic partial-line test. |
| Concurrent mutations change cross-line ordering. | Document completion order and require client sequencing; preserve order inside batches. |
| Oversized batch failure leaves per-ID promises pending. | Explicit single-envelope refusal contract, matching upstream; admitted batches correlate every ID. |
| Arbitrarily long input exhausts memory. | Accepted existing residual; do not market the concurrency cap as total memory protection. A future byte cap needs refusal/drain semantics. |
| Slow stdout delays deadlines and exit indefinitely. | One owner and bounded exchange count; terminal write/flush errors; explicit lack of wall-clock drain bound. |
| Panic loses all in-flight replies. | Accepted cost of rejecting spawned task isolation; handle all expected parsing and transport errors without panic. |
| Sixteen waits starve a later ping or cancellation. | Named backpressure limit; prove progress below saturation and release at capacity. No priority or cancellation promise. |
| Futures schedule differently under instrumentation. | Causal rendezvous/registration tests, independent inboxes, no timing margins or sleeps; preserve helper deadlines. |
| Reply framing changes or produces extra blank lines. | Compact JSON values into one write path, exact line counts and multiline-body fixture. |
| New lines evade exact coverage through harness placement. | 0020 placement and 0035 gates; justify any cold-path harness, execute every seam. |
| Git or toolchain sandbox prevents delivery. | Save files and exact errors; do not fabricate a commit, bypass permissions or report green gates. |
