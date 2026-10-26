# Issue #151 part 3 — fixture-http MCP metadata (intake)

Run: epic-164-issue-151-make-the-fixt-547fc316.
Phase: intake; framing only. Result: resolved (actionable, not implemented).

## Goal

Make the real stdio fixture-http helper accept and discard standard MCP
metadata in all supported locations, and give GET bodies the requested defined
answer, while preserving its one-tool surface, closed tool inputs and bounds.

## Authority and spot-checks

Read docs/house-rules.md, the decision index, frozen 0001–0003, accepted 0055
(standalone eval crate and real subprocess proof), accepted 0075 (private inputs
and value-free diagnostics), 0029/0044 (validated wire inputs and closed tool
arguments), and proposed 0083 (wrapper/helper boundary). This is compatible
helper repair under the supplied operator direction; no accepted ruling needs
amendment and no decision is accepted here. Keep candidate bundle routing.

The entire supplied issue151 evidence packet passed SHA256SUMS when checked
from its own directory. README.md digest:
bad2a4ba38b3c879d371b2657e48de29d452502f56ad9c5577d212f4359f824e.
operator-rulings.md digest:
5b8da87b897306a313f673b134edd1a56e0f477117a07618e3ec345449a6c6f9.
The initial root-relative checksum command used the wrong base directory;
the corrected packet-directory check passed without exceptions. The README's
prior-runs section is empty. The supplied smoke records Claude authentication,
the intended two MCP servers, real calls, and the params._meta refusal; those
are operator evidence, not newly reproduced live measurements. No required
local input is missing. No GitHub, CI, journal or sibling worktree was read.
Evidence paths are plain provenance text, never tracked Markdown links into
untracked delivery state; tracked authority lives in docs/rulings/epic-192.md.

History: intake starts at c5807fc (#262), which introduced the helper and tests.
Spot-checked evals/src/fixture_http.rs: Call and Arguments deny unknown fields;
Call lacks _meta; Helper::rpc dispatches initialize, ping, tools/list and
tools/call; Helper::call refuses any present GET body and maps failures to the
generic fixture_request_refused tool result. serve/line cap stdio input at
64 KiB; responses cap at 1 MiB, with existing deadline, origin, identity,
no-proxy and no-redirect protections. evals/tests/wrappers.rs already launches
the real helper and owns a real loopback HTTP server. evals/Cargo.toml declares
an independent workspace, so root tests/coverage alone cannot prove this fix.
The supplied three schema snapshots all allow request params metadata;
2025-06-18 and 2025-11-25 explicitly permit additional metadata keys, and
notifications also carry metadata. Receiver progress notifications are optional.

Frozen guarantees nearby: none is modified. 0001 synchronous accept/store,
0002 subscribe-before-check and 0003 oldest-only pruning remain outside this
change, as does 0005 ingest extraction. Only evals/ may change (plus the
engine-required untracked result file). Do not change root code, gates,
manifests/locks, dependencies, allowlists, routing, scoring or decision status.
Do not run paid trials or authenticated model calls.

## Acceptance criteria

1. Add failing regression tests first against the actual fixture-http stdio
   subprocess and a real loopback HTTP fixture. Retain the pre-fix command,
   nonzero test exit and relevant assertions as red evidence; the same tests
   pass after repair. Cover all three negotiated revisions: 2025-03-26,
   2025-06-18 and 2025-11-25. No mocked helper/RPC substitute proves this claim.
2. For each revision, initialize with permitted params._meta negotiates the
   requested version; ping, tools/list and tools/call accept permitted request
   metadata; notifications/initialized with permitted metadata stays silent.
   tools/call with params._meta.progressToken (both integer and string tokens)
   reaches the HTTP fixture and returns the normal successful tool result.
   Cover omission, empty metadata and spec-permitted extra/nested metadata.
   Metadata contents are ignored: no progress emission is required, and
   distinctive metadata sentinels appear in neither replies, stderr nor the
   captured HTTP request (path, headers or body).
3. The tools/list descriptor still advertises exactly eval_http. Unknown tools,
   unknown tools/call params other than _meta, and unknown eval_http argument
   keys retain their existing refusal. In particular arguments._meta is not
   promoted into params._meta or admitted as a tool argument. Wrong-shaped or
   missing required tool arguments still refuse. Negative tests verify no HTTP
   effect for refused calls. Do not introduce unrelated stricter validation on
   methods that were previously permissive, or general protocol conformance.
4. GET with body "" has one documented, tested outcome: either a successful
   request with no HTTP body, equivalent to omission, or result.isError=true
   with a safe diagnostic naming the GET-body problem. It never becomes a
   JSON-RPC error. Other refused GET bodies likewise have a tool-level error
   naming the problem; do not loosen their existing refusal. Preserve valid
   POST JSON bodies and the existing safe generic handling of unrelated errors.
5. Existing identity injection, app-relative origin/path and caller-header
   restrictions, no proxy/redirect behavior, 64 KiB input and 1 MiB response
   bounds, deadline and byte-preserving response behavior retain their tests
   and assertions. Oversized metadata remains within the existing total input
   cap; add a real-stdio negative control proving metadata cannot bypass it.
   No new tool, arbitrary HTTP target, credential forwarding or metadata echo.
6. Standalone eval fmt, locked all-target clippy, wrappers and full eval suite
   pass. Every local house gate below passes on the committed candidate,
   including exact coverage with zero missed lines, branches and functions and
   its selftest. No coverage exclusion or denominator/floor reduction. Record
   command exits and logs; failures remain outstanding work, not green proof.
7. Commit all tracked changes unsigned; the tracked worktree is clean before
   delivery returns. No push, merge, tag, publication, issue closure, signing,
   sealing or decision acceptance. Intake commits only this framing and its
   receipts; implementation and its regression proofs belong to later phases.

## Verify gate commands

Run offline (CARGO_NET_OFFLINE=true), install nothing. Check the local engine
pin and shim integrity first; execute pinned script owners rather than shims,
per 0050. The public house command names resolve to these pinned owners.

```bash
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum --check --strict
sha256sum --check --strict bundles/self/scripts/shims.sha256
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
bash bundles/self/scripts/coverage-gate.sh
bash bundles/self/scripts/coverage-gate.sh selftest
bash bundles/self/scripts/publish-guard-test.sh
bash bundles/self/scripts/seal-branch-test.sh
cargo deny check licenses
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
```

Compile must emit no policy-sweep warning. Fail closed on native-tls absence:

```bash
out=$(cargo tree -i native-tls 2>&1); code=$?
if [ "$code" -eq 0 ]; then exit 1
elif printf '%s' "$out" | grep -q 'did not match any packages'; then exit 0
else exit 1; fi
```

The independent eval crate needs its own checks, with the focused wrappers
target used for both red and green real-stdio regression evidence:

```bash
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path evals/Cargo.toml --locked --test wrappers
cargo test --manifest-path evals/Cargo.toml --locked
```

No cargo bench is required: no performance contract changes. Root exact
coverage remains mandatory and does not measure this standalone eval crate.
CI owns macOS, chart/baseline/history, Docker, network RustSec and privileged
port-80 proofs listed by the charter; #201 owns macOS eval gating. The operator
owns live settings, authenticated wrapper interoperability, paid baseline,
publication and acceptance. Namespaces remain deferred under #221. These are
owned external proofs, not extra criteria for this offline seat. This result
frames only part 3 and does not claim the baseline or all of #151 is complete.
