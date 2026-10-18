# Implementation plan: HTTP plane edges

Implements [spec.md](spec.md) and proposed
[0043](../../docs/decisions/0043-mcp-origin-boundary.md).
This is an implementation handoff, not a claim that its tests have run.

## Technical context

One Rust crate using Tokio, axum and atomics. api.rs owns the outer counting
middleware, the inner whole-plane header guard, AppState's optional bound
loopback port and the four registry renderers. lib.rs derives the posture
from the listener's local address. MCP deliberately receives String and
dispatch owns JSON-RPC parse errors (0029 §2). tests/common starts real
servers; tests/binary provides isolated serve children and process cleanup.
No new production dependency, data migration or configuration is required.

There is no .specify/templates/plan-template.md in this checkout. This plan
provides the requested technical context, constitution check, project
structure, research, data model, contracts and quickstart directly.

## Constitution check

0001/0002/0003/0005's store, wait, prune and extraction ordering stay unchanged.
0030's loopback boundary stays intact except #92's Host omission correction;
0043 explicitly proposes the off-loopback amendment. It remains proposed.
0025's supplied-invalid versus absent distinction applies to ports. Tests
obey 0004/0020: real sockets for wire behavior, deterministic branch-stable
unit tests inline, no global counter reset or sleep synchronization. A sibling
cfg(test) harness is justified only by a concrete cold failure path. Keep
0035's exact coverage measurement, no exclusions, new feature flags or gate
edits. No dependency allowlist change; no production changes outside the API
boundary are anticipated.

## Project structure and files touched

| Path | Responsibility |
|---|---|
| src/api.rs | TotalGuard constructor/lifetime, label encoder, off-loopback Origin check, Host default, local validators, measured unit tests and accurate comments. |
| tests/binary.rs | Isolated real-HTTP total regression and readiness-driven release. |
| tests/api.rs | Host/default and off-loopback HTTP matrix, whole-plane/OPTIONS checks, metrics label round trip. Public AppState permits a real router fixture with policy 80. |
| tests/mcp.rs | Body-type and JSON-RPC compatibility where not already pinned in tests/api.rs. |
| tests/common/mod.rs | Minimal reusable raw-header/response support and strict emitted-subset metrics reader only if shared. |
| tests/stdio.rs | Existing real bridge suite is a required regression; change only if a missing compatibility case is needed. |
| README.md, src/static/llms.txt | Network posture, proxy/forwarding costs, observer contribution; reconcile stale inert-guard language. |
| docs/decisions/README.md | Index proposed 0043 in implementation, not this four-file design commit. |
| specs/http-plane-edges/{spec,plan,tasks}.md and docs/decisions/0043-mcp-origin-boundary.md | Exactly four design artifacts; dated clarifications and proof receipts later update their owning requirements. |

No edits to accepted 0030, bundles, schema, Cargo manifests or lockfiles are
planned. Preserve actual bind derivation in lib.rs; test it through run_on.

## Research and resolved choices

Source confirms the bare TotalGuard construction has only a decrement;
registry_rows uses Debug; middleware already wraps all routes; and the
default authority helper requires explicit ports. An invalid IPv6 suffix is
currently collapsed to no port, which must not gain admission when Host's
default is added. The port must remain Option until syntax validation succeeds.

The route audit resolves robustness's uncertainty: chaos/webhook clear are
DELETE, seed uses JSON POST, MCP uses String POST, and UI/compat add no POST.
Select Origin validation for explicit boundary and curl compatibility. The
spec rules against both the media-type gate and omitted-port wildcard. Using
the Origin scheme for both absent defaults handles ordinary TLS termination
without trusting forwarded headers. It cannot establish the external target
scheme; document this limitation instead of claiming full same-origin proof.

## Data model and implementation approach

No persistent state changes. Retain the process-global atomic and registries.
Give TotalGuard a take constructor that increments with Relaxed ordering,
then returns the guard. Use it exactly once in count_in_flight; keep Drop as
the sole decrement. Allow a borrowed atomic lifetime if needed for fresh
local-atomic tests, rather than allocating or resetting a test global. Keep
counter middleware outermost and its response-production lifetime unchanged.

Add a pure prom_label encoder beside registry_rows. Use a character loop or
three correctly ordered replacements, then explicit surrounding double
quotes at render. Unit fixtures verify exact output bytes; all four families
must continue through registry_rows. Do not reuse the encoder in the parser.

Factor authority splitting only as far as it preserves loopback behavior.
Represent host text plus optional validated explicit port. Distinguish an
empty suffix from ':', garbage after ']', an overflowing value and an
unbracketed colon sequence. Off-loopback needs a strict serialized-Origin
validator; it accepts http/https only, no URL suffixes or userinfo. Do not
replace the old loopback Origin helper with this stricter policy: its legacy
scheme/path acceptance is intentionally unchanged by this commission.
Reuse splitting mechanics where possible, but keep admission policy at the
caller. The new Host-default branch must accept only syntactically omitted
ports, never recover malformed supplied values. Pin the established loopback
accepted set before factoring to avoid accidental broadening or narrowing.

In origin_guard, retain the Some(bound) Host-first branch and existing
messages. Its Host call gets default 80; its Origin call still requires a
port. In the None branch, absence of Origin goes directly to next; presence
requires exactly one valid Origin and Host, matching normalized authorities.
Reject invalidity via ApiError 403 with the spec's exact sentence before next.
No media-type check, new state, CORS layer or forwarded-header interpretation.

## Contracts and proof design

AC1 uses a fresh atomic, explicit drop and deterministic future destruction
after the guard has been installed; unwind uses catch_unwind around an owned
guard. Test a guard lifetime, not an unproven claim that every client disconnect
immediately cancels every HTTP handler. No sleeping or counter reset.

AC2 uses one child and one parked await. Sequentially scrape until its labeled
request and awaits rows establish readiness, under a failure deadline. Check
the unlabeled sample from that same response is 2. Deliver real SMTP to satisfy
the wait, join the response, and observe return to raw 1 on subsequent quiet
scrapes. A cleanup guard stops and reaps the child on success and failure.
Avoid other simultaneous HTTP operations while asserting the exact total.
The child isolates its static from the parallel suite without serializing it.

AC3's independent test-only reader handles comments/TYPE lines, metric names,
quoted labels with only the three legal escapes, sample separators and numeric
values in this endpoint's emitted subset. It must parse the entire response,
reject malformed input and recover labels, not just search for substrings or
check for bad escapes. Its fixtures include a known valid independent sample
and the invalid examples in AC3. Park the encoded-name wait and poll its
decoded awaits row, then satisfy/join it. The request registry intentionally
uses raw path segments, so it need not have the same decoded key.

AC4–AC8 bind through start_with/run_on to 0.0.0.0:0; dial 127.0.0.1 at the
reported port. Assert the configured/listening address is nonloopback, not
merely a manually disabled guard. Use fresh server state for chaos probes,
probability 100 and direct before/after configuration comparison. Clear any
accepted control probe before other operations. Raw TCP helpers must permit
an exact Host, repeated headers and inspection of response headers without
silently appending another Host or Content-Type. Distinguish HTTP parser
rejections from middleware refusals. Pure inline validator cases reach every
malformed input branch, including cases HTTP will reject before the router.

AC9 retains the three named existing tests. AC10 serves the public real
router on an ephemeral socket with fresh AppState and Some(80) then Some(8025).
This is a real HTTP policy test, not a claim to bind privileged port 80.
Exercise lib.rs's real posture derivation separately via the loopback and
0.0.0.0 startup fixtures. Keep all portless-Origin assertions explicit so
defaulting Host cannot silently change Origin semantics.

Capture red-before/green-after evidence for #88's two defects, #89 and #92.
New wire regressions can first run against unchanged production behavior;
otherwise temporarily restore the smallest original faulty behavior and
record the failing assertion. A compile failure is not evidence. Restore all
mutants before gates; retain commands and observed assertions in run-local
evidence, with a concise receipt alongside the acceptance criteria.

## Quickstart and validation

Use a feature branch, then targeted api/binary/mcp/stdio suites before the
full gates. Run these exact gate commands, without a TMPDIR override:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test --workspace --locked --test api --test binary --test mcp --test stdio
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh
cargo deny check licenses
```

Require exact nonzero 100/100/100; no exclusions or floor changes. All new
measured lines need execution and behavioral assertions, not coverage alone.
Record any gate unavailable in the environment honestly.

For the design seat, validate four nonempty files, the Acceptance Criteria
and Risks headings, ordered task checkboxes, all relative links and proposed
0043. Stage and commit exactly those four paths with an imperative subject
and why body; never push. Do not add the decision index in this commit.
If the environment refuses .git writes, preserve a patch and disclose that
the artifacts remain uncommitted; do not redirect Git metadata or claim a
commit. Implementation/landing remains the later sequence's work.

## Risks

| Risk | Mitigation / accepted cost |
|---|---|
| Scrape counts itself or parallel servers mix totals. | Explicit raw 1/2/1 contract; fresh child process; local atomics for lifecycle tests. |
| Readiness poll or disconnected wait becomes a timing guess. | Observe labeled rows, deliver/join, use real failure deadlines and cleanup guards. |
| Renderer and permissive test parser share the same mistake. | Independent grammar/round trip, invalid fixtures and byte pins; no production parser dependency. |
| Defaulting ports admits malformed suffixes. | Preserve absence as Option; validate supplied ports and bracket tails before applying defaults. |
| Refactoring changes the loopback Origin accepted set. | Keep its policy distinct; unchanged existing suite plus explicit scheme/path/port pins. |
| Proxy rewrites authority or strips a nondefault port. | Document preserving original Host; refuse ambiguity. No forwarded-header bypass. |
| Host lacks an external scheme. | Compare normalized authorities only; preserve ordinary TLS ingress; document absence of full origin authentication. |
| Off-loopback no-Origin requests and DNS rebinding remain possible. | Document network trust and unauthenticated peers; no claim of an auth boundary. |
| Port-80 Host works but a portless loopback Origin still refuses. | State the limitation in spec/ADR/docs and pin it; broadening Origin is explicitly out of scope. |
| Coverage-driven harness migration hides tests or production. | Follow 0020's cold-path rule and 0035; leave deterministic tests inline. |
| Sandbox blocks Git, signing or gates. | Preserve concrete artifacts and patch, report exact blocked operation; do not assert delivery or green gates. |
