# Implementation plan — checked usize proof

Implements [spec.md](spec.md). This follow-up refines the remaining AC7/AC10
work in [RPC edge contracts](../rpc-edge-contracts/spec.md); its completed
implementation and dated history remain evidence.

## Technical context

Single Rust crate with existing serde_json staging and real-server MCP tests.
No dependencies, schema, persisted data, public API, feature flag or new module.
The checkout has no plan template; the required template sections appear here.
Baseline: dd76dfe54e31a7ff4fa8e2d4b90f2377968056a8.

Use candidate-local .forge/bin/brokkr after confirming SHA-256
ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f
against bundles/self/tooling.json and this task. Chief preflight matched it.
Resolve that candidate path absolutely for compile checks. Historical engine
paths and digests in the prior plan are historical receipts, not current
execution instructions. No journal fabrication or tooling installation in
hermetic verify.

## Constitution check

Preserve 0001–0005, 0029 staging, 0044 tool bounds, the dated 0047 ID amendment,
0020 measured-inline ownership, 0035/0050 exact proof and 0056 unsigned commits.
0062 remains proposed. This refactor changes no normative behavior, so it
needs no new ADR or decision-index edit. The engine triage under 0057 retains
the four review seats named in spec AC7. This design seat does not dispatch
later implementation/review or claim those results.

## Research and resolved questions

The retained coverage summary and source isolate a target-cold map_err closure,
not a missing runtime guard. A string-only extraction leaves the closure cold.
One concrete Result adapter removes that separate owner and gives host tests
access to actual production translation, without changing conversion.

Simplicity found the unconditional u64::MAX successful-page fixture.
Chief i686 metadata probes additionally confirmed that u64::from(usize::MAX)
in both cfg-32 witnesses fails E0277 and that the checked fixture conversion
compiles. Correct all three fixtures; the latter two are test compilation
repairs, not a new production conversion policy. Both panels' runtime
requirement stands. Installed i686-unknown-linux-musl alone does not establish
a usable C linker/SQLite toolchain or executable runner.

## Project structure and files

| File | Planned work and proof |
|---|---|
| src/mcp.rs | Extract one private concrete result mapper; keep arg_usize classification and checked conversion. Add measured inline Ok/Err and ordinary staging controls only where absent. Correct paging success maximum and cfg-32 max construction. |
| tests/mcp.rs | Correct only the existing cfg-32 witness's max construction; retain its real HTTP assertions and existing caller coverage. |
| specs/usize-proof/spec.md, plan.md, tasks.md | Current scoped contract, design evidence and later truthful progress. |
| specs/rpc-edge-contracts/plan.md and tasks.md | During implementation only, append dated current-engine/proof follow-up and update remaining proof status solely from receipts; preserve earlier dated records and satisfied criteria. |
| .forge/ run-local evidence | Commands, source identities, exits, test counts, red/restored-green and coverage receipts. Never commit run-local evidence or modify historical receipts. |

The design commit contains only the three usize-proof artifacts. Updating old
spec artifacts belongs to implementation, after the design check, so the
chief's HEAD touches exactly one spec directory. src/stdio.rs, tests/stdio.rs,
tests/binary.rs and all other production owners are preservation boundaries,
not planned edits. Dependencies, gates, shims, recipes and decisions stay intact.

## Data model and contracts

No data migration. Keep the current Result<Option<usize>, (i64, String)>
staging type. The private mapper accepts &str and
Result<usize, std::num::TryFromIntError>; a direct match returns Ok(Some(n))
or Err(refusal(key, "a non-negative integer that fits usize")).
arg_usize retains absence and as_u64 classification, calling the mapper with
usize::try_from(n) in its numeric arm. Keep refusal as the only wire formatter.
Do not allocate an error String on success.

Inline tests pass an actual portable TryFromIntError from u8 conversion to
the mapper, asserting exact offset/count sentences and successful values.
This is an error-translation test, explicitly not simulated usize overflow.
Real target conversion is proved separately by the cfg-32 staging and HTTP
tests. The successful paging fixture uses json!(usize::MAX); the two
cfg-32 maxima use u64::try_from(usize::MAX).unwrap().

## Implementation and proof sequence

1. Preserve baseline source identity and raw receipt. Reuse the chief's
   fresh unchanged-source reproduction below when source identity matches;
   otherwise rerun canonical coverage before edits. A refusal is not a pass.
2. Fix the three test fixtures and add focused mapper/staging tests in the
   existing measured module. Implement the one production mapper and its
   call from arg_usize; no new validation owner.
3. Run host MCP unit and real HTTP suites. Verify all arg_usize callers retain
   validation before scans/admission; reuse existing zero, cap, default and
   wrong-type controls rather than duplicate protocol projects.
4. On a supported installed 32-bit target/runner, execute both overflow
   witnesses and corrected paging test. Use a temporary narrowly scoped
   unchecked-cast mutation to prove overflow sensitivity. A compile failure
   is not behavioral red. Restore exact source bytes and rerun green.
   Unavailable infrastructure leaves AC4 open.
5. Run the full canonical local inventory and verify-owned proof chain on the
   restored final candidate; exact coverage and selftest both must pass.
   Preserve --locked and fail-closed dependency checks. Verify runs offline
   and installs nothing. Required CI/supported-host receipts remain separate.
6. Reconcile remaining proof boxes and dated execution identity in the prior
   handoff without rewriting historical results. Commit implementation
   unsigned, then let the engine's protected review dispatch its selected
   four providers. No push, merge, tag, publication or issue closure here.

## Quickstart and validation

From this candidate root, after implementation:

```bash
sha256sum .forge/bin/brokkr
cargo test --locked --lib mcp::tests
cargo test --locked --test mcp
cargo test --locked --test store_contract_docs
```

For the currently installed target, only when its toolchain and runner work:

```bash
cargo test --locked --target i686-unknown-linux-musl --lib usize_staging_accepts_max_and_refuses_overflow_on_32_bit
cargo test --locked --target i686-unknown-linux-musl --test mcp mcp_search_offset_refuses_values_beyond_32_bit_usize
cargo test --locked --target i686-unknown-linux-musl --lib search_pages_with_offset_and_keeps_the_total
```

Record actual target and runner if a different supported 32-bit target is
used, and assert nonzero executed counts for each command. Metadata probes
are insufficient. Do not turn cfg-skipped host execution into runtime proof.

The single gate inventory remains
[house rules](../../docs/house-rules.md#gates--part-of-every-change).
Use the canonical pinned scripts after shim integrity checks; verify-seat
also owns design-check selftest and delivery tamper-boundary proof. Engine
compile must match the current digest and emit no policy-sweep warning.
Do not infer review, 32-bit or CI success from local gate outcomes.

Validate the final chief commit with
`bash bundles/self/scripts/design-check.sh check .`; the sequence executes
its pinned copy independently. Structural design validation is separate from
production acceptance.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Host mapper passes even after checked conversion is removed. | Keep actual 32-bit staging/HTTP witnesses and behavioral unchecked-cast mutation plus restored-green receipts. Runner availability can block delivery. |
| The old 32-bit witnesses never compiled. | Fix both E0277 max fixtures and the target-dependent paging success case; compiler probes are labeled compile-only and followed by actual runtime. |
| Extra helper changes error text, fields or successful-request cost. | Reuse refusal verbatim, test offset and count, use a direct match with no eager error allocation and retain all caller controls. |
| Target-cold closure survives or new measured code remains cold. | Move the complete mapping into the production-used helper; exact line/branch/function equality and selftest, no denominator workaround. |
| Refactor delays validation until after waiter registration. | Keep call-site ordering unchanged, run existing no-waiter and real-server tests; Store still subscribes before reading. |
| Current design erases earlier completion or missing evidence. | Preserve old dated records; update only remaining proof statuses from receipts. Keep macOS stderr, port-80 and CI-owned obligations separately open. |
| Broad robustness audit expands this repair. | Preserve failure boundaries without touching storage, SMTP, webhook, TLS, concurrency or resource policy. Fresh scope evidence is required for expansion. |
| Wrong engine or reduced review silently changes delivery. | Verify candidate-local digest, keep pinned copies and intake engine classification; provider failure stops without substitution. |

## Design evidence boundary

No product repair, actual 32-bit runtime or review pass is claimed here.
The retained gate failure and chief fixture probes establish the design's
premise. Design-tree gate observations will be recorded below separately
from the final implementation's required clean proof.

## Design-tree validation — 2026-10-24

Ran the complete local house inventory with CARGO_NET_OFFLINE=true against
unchanged production/test source at dd76dfe and these draft documents. Shim
integrity, fmt, locked all-target clippy, locked workspace tests, digest-bound
self-bundle compile, coverage selftest, design-check selftest, delivery
tamper-boundary proof, publish/seal proofs, licenses and fail-closed native-tls
absence all passed. The native-tls command returned 101 with the expected
“did not match any packages” diagnostic; that is the absence proof.

Exact coverage reproduced the same refusal: 13078/13079 lines, 766/766
branches, 1444/1445 functions. Fresh evidence is
target/coverage/run.XeeFre/{lcov.info,coverage-summary.json}; source identity,
commands, exits and logs are under .forge/design-usize-proof/gates/. The
workspace client's 300-second response timed out, but the process completed
and its saved results.json and logs were subsequently read back. No check
is inferred from the timeout. The required implementation coverage pass,
actual 32-bit execution, external host/CI proofs and protected review remain
outstanding. This reproduction satisfies the baseline inspection task only.

## Implementation validation — 2026-10-24

The deterministic check passed on the committed design at `a4a43f4` using the
candidate-local brokkr binary with SHA-256
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`. Before
source edits, the chief's unchanged-source hashes for `src/mcp.rs` and
`tests/mcp.rs` matched this candidate, so its canonical coverage reproduction
was reused as the baseline refusal. The implementation preserves checked
`usize::try_from`, extracts the concrete result mapper, adds measured inline
controls and corrects the three target-aware fixtures.

Focused host unit tests passed (10), the real-server MCP suite passed (44),
and `store_contract_docs` passed (19). Final exact coverage is 13121/13121
lines, 766/766 branches and 1447/1447 functions at
`target/coverage/run.gmQPcl`; coverage selftest passed. Shim integrity, fmt,
locked clippy and workspace tests, self-bundle compile, publish/seal proofs,
licenses, design-check selftest, delivery tamper-boundary proof and the
fail-closed `native-tls` absence proof passed. The per-command logs are in
`.forge/usize-proof/`.

The installed target is `i686-unknown-linux-musl` on an `x86_64` host. The
three exact filtered commands in this plan were attempted; each exited 101
before test execution because `musl-gcc` is absent while building native
dependencies. They ran 0 tests and produced no binary. No actual 32-bit runtime,
unchecked-cast behavioral red, source restoration rerun or 32-bit green is
claimed. AC4 remains open. The engine-selected protected reviewer receipts
are also not present in this implementer phase; no reviewer substitution is
made. Actual 32-bit execution, protected review, macOS stderr, privileged
port-80 and CI-owned chart/Docker/RustSec/tag-release evidence remain separate
open obligations.
