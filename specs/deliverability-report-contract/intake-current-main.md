# Intake — issue #252 on current main

Run: `epic-251-issue-252-implement-the-22ac6e57`; phase: intake; 2026-10-26.
This framing supersedes the earlier intake's MCP prerequisite. No implementation
is performed by this seat.

## Goal

Implement a deterministic, versioned typed deliverability report lazily from
stored bytes with a never-stale cache, shared by REST, current-main MCP, the
message UI and existing assert/wait predicates, independently of epic #164.

## Authority and orientation

The issue packet README.md and operator-rulings.md both verified against their
directory's SHA256SUMS; the prior-outcomes section is empty. The operator's
current brief supplies the prior run's IMPL-BLOCKED outcome and explicitly
withdraws its #153 prerequisite. Local history confirms the contract design at
`d316b2c` and earlier prerequisite record at `42ff58c`; starting HEAD is
`0e0d51c`. No GitHub, CI, journal or sibling worktree was read.

Read the house rules, decision index, 0001–0003/0005, and relevant assert/wait,
admission, extraction, offload and coverage rulings. Spot-checked synchronous
`Store::insert`, panic-preserving `Store::offload`, `Store::wait_for`'s initial
and Closed subscribe-before-scan, `Email.raw`, API assert/await, message_view
and current MCP helpers. `src/mcp.rs` advertises 2025-03-26 and text_result
puts JSON in one text block. The local engine digest is exactly
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.

Current active draft spec, plan, tasks and proposed 0085 are corrected under
the operator's explicit instruction. Earlier outcomes/receipts remain historical.
No design or completion dependency remains on 0079/0080/0082 or #152/#153.
When #153 lands, report structuredContent/outputSchema is a later, small
follow-up through its shared owner. All framing inputs are locally present.
Keep 0085 proposed and this candidate's bundles/recipe routing unchanged.

## Checkable acceptance criteria

The detailed field names, ordering, limits and paired witnesses remain in
[spec.md](spec.md) AC1–AC10 and [tasks.md](tasks.md). Completion requires:

1. **Typed/versioned model:** fixtures pin rule_set_id, stable rule_id,
   info/warning/error, all header/MIME-part/link location shapes and fix_hint,
   canonical ordering/deduplication, empty reports and the demo rule's exact
   bytes/size/malformed-input behavior. Only demo.marker.v1 ships here.
2. **Determinism:** identical raw bytes, exact effective config and version
   serialize identically across independent evaluators, cold/warm reads and
   restart despite arrival metadata changes. Config/version changes cannot
   reuse old results. No ambient clock, DNS, network or external scorer.
3. **Ingest preservation:** owner-local instrumentation proves zero report
   evaluation on SMTP, seed and restore. Real SMTP DATA 250 permits immediate
   byte-exact retrieval before analysis. Reporting leaves extracted fields and
   persisted rows unchanged; SMTP progresses while report workers are held.
4. **Never-stale bounded cache:** deterministic ownership tests pin same-key
   coalescing, config/version identity, same-id replacement, clone/mutate and
   recovered-owned reinsertion, deletion/clear/oldest pruning and restart.
   Selected pre-removal snapshots may finish; new lookups cannot resurrect mail.
   Weak entries retain no raw mail and cannot change retention order.
5. **Resource/failure truth:** barriers pin four-worker and 128-slot admission,
   eviction/all-pinned refusal, findings/JSON limits, cancellation/panic guard
   lifetime and retry. Incomplete/failed analysis never yields clean success
   or a partial exact count. Shutdown/deadline outcomes match the spec.
6. **REST:** real servers prove report JSON, missing-message 404, sanitized
   503s, closed query validation before analysis and served OpenAPI parity.
7. **MCP on current main:** real HTTP/stdio prove the same report as REST in
   one JSON text block, isError=false on success, existing operational errors,
   -32602 malformed arguments, required-id/closed input descriptor parity and
   unchanged initialization. No report outputSchema/structuredContent. The
   later #153 follow-up is documented in 0085/spec, not required for completion.
8. **UI:** real message pages show report fields, empty and unavailable/failed
   states; adversarial fixtures prove escaped text and inactive link locations.
   List rendering never warms reports; reporting does not alter stored mail.
9. **Assert/wait:** real REST/MCP prove inclusive severity thresholds, exact
   assert including zero, minimum-one wait, filter-before-page, newest-first
   order, preserved totals, mismatch/timeout/shutdown and invalid-input refusal.
   Deterministic owner seams prove subscribe-before-scan, delivery during
   analysis, lag and Closed recovery with no sleep synchronization.
10. **Extension/documentation/delivery:** a pure fixture collector with explicit
    immutable additional inputs proves shared ordering/serialization/thresholds
    and cache identity. #253 owns DNS policy, expiry/refresh/wakeup; this issue
    adds no resolver, real authentication/content/spam rules or dependency.
    Living docs match the contract, 0085 stays proposed, and every local house
    gate plus verifier proofs passes on the committed implementation with exact
    zero missed lines/branches/functions and the full coverage selftest.

Nearest frozen guarantees are 0001 (analysis stays off SMTP), 0002 (predicate
waits subscribe before scans) and 0003 (cache lifetime preserves oldest-only
pruning). 0005's ingest extraction remains unchanged. The additive report has
a proposed decision and requires no weakening or accepted semantic amendment.

CI owns macOS, network RustSec, chart/history/Helm, Docker, privileged port-80
and tag release-rate proofs. Live GitHub settings, namespace support, decision
acceptance, sealing/publication/merging remain CI/operator-owned; none is an
extra local criterion. No push, merge, tag, publication or issue closure.

## Verify commands

Use real-server suites, no mocks, plus deterministic owner tests under 0020:

```bash
cargo test --locked --lib report_
cargo test --locked --test api
cargo test --locked --test mcp
cargo test --locked --test stdio
cargo test --locked --test ui
cargo test --locked --test persistence
cargo test --locked --test burst
cargo test --locked --test p2
cargo test --locked --test rate
```

T1–T14 assign concrete new witnesses; these commands do not claim they exist
yet. No throughput claim or new benchmark is required; rate/burst retain the
existing acceptance/count proofs. Every house gate and verifier addition:

```bash
export CARGO_NET_OFFLINE=true
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
out=$(cargo tree --locked -i native-tls 2>&1); code=$?
if [ "$code" -eq 0 ]; then
  printf '%s\n' "$out" >&2; exit 1
elif printf '%s' "$out" | grep -q 'did not match any packages'; then
  :
else
  printf '%s\n' "$out" >&2; exit 1
fi
```

0050 requires fixed-shim integrity checks and execution of canonical pinned
bundle scripts instead of unchecked candidate wrappers. Run offline, install
nothing. Exact coverage and its selftest are separate mandatory proofs. The
fresh intake baseline receipt will identify the committed framing tree; it is
not proof of the future implementation. Verify reruns all gates after code lands.
