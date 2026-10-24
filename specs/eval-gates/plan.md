# Implementation plan — eval gates

## Technical context

One Rust service crate plus the already accepted standalone `evals/`
workspace; Unix subprocess lifecycle on Linux/macOS; Tokio, serde_json and
existing test helpers; Bash/jq for pinned verification and GitHub Actions.
No new dependencies. Root coverage remains exact 100% lines, branches and
functions; evals remains outside that denominator. Implementation starts from
`dc1822a45e4d2ae8326b44baf22818187a8320e9` plus this design commit.

The checkout has no plan-template file under `.specify/`; this plan supplies
the requested technical context, constitution check, project structure,
research, data model, contracts, quickstart and risks directly. No extra
research/data-model artifact is necessary for this bounded change.

## Constitution check

Implement accepted 0055's conditional wiring under 0050's pinned proof
ownership. No accepted ruling or public contract is amended; no ADR is
needed. Preserve 0001–0005, 0043, offline/no-install verification, all charter
gates, protected review and security hold. Tests use real servers for protocol
acceptance. Pure artifact/lock/decoder inputs test rejection seams, not mail
acceptance. New root test cold paths follow decision 0020. Unsigned commits
follow 0056, superseding stale signing instructions in older plans.

## Research and resolved choices

The two panel files and intake under `.brokkr/state/` are run-local evidence.
Inspection establishes these implementation facts:

- CI tests only the root workspace; eval dependencies and test targets require
  explicit manifest-path invocations. Keep quality and both OS test jobs.
- `verify-seat.sh::run` launches a new shell per command. A transient export
  inside one invocation does not reach another; explicitly carry binary
  provenance across the dependent root/eval test steps.
- `delivery-proof-test.sh` already owns isolated candidate fixtures and
  tamper controls. Extend its proof machinery, not the verifier algorithm in
  an unrelated test script. Avoid recursive verify-to-proof-to-verify calls.
- Root-test Cargo output contains both artifacts and test output. Keep only
  valid artifact JSON for selection, retain all diagnostics, and check Cargo's
  exit first. Identify the root package from canonical manifest metadata,
  not metadata array order, target filename searches or package name alone.
- `runner::drain` currently chunks reads at 8192 bytes but forwards chunks
  through an unbounded queue. Retained server receivers outlive announce;
  agent receivers are dropped. Existing address parsers both return two
  `SocketAddr`s, allowing a small typed one-shot mechanism.
- MCP client tolerance and exact current server shape differ. Assert the
  latter on real replies; extract only existing decoding into private pure
  functions to exercise malformed inputs without adding protocol strictness.
- 0055 and its index already record acceptance. The obsolete claims are in
  current guidance. The exact external #149 erroneous wording was not
  supplied or found in inspected repository text; record the canonical path
  and explicit external follow-up instead of inventing a change.

## Project structure and files

| Owner | Change |
|---|---|
| `.github/workflows/ci.yml` | Eval fmt/lint in quality; root artifact capture and full eval tests in both test jobs; preserve publication dependencies and finite deadlines. |
| `bundles/self/scripts/verify-seat.sh` | Same mandatory checks, root binary provenance, explicit path propagation, diagnostics and success notes; offline/pin/shim refusals retained. |
| `bundles/self/scripts/delivery-proof-test.sh` | Focused eval dispatch proof mode and existing fixture utilities; default tamper proof remains nonrecursive. |
| `tests/delivery_constitution.rs` | Structural wiring/resource/CI publication guards that supplement executed proof. |
| `docs/house-rules.md` | Extend the sole gate inventory and explain standalone eval/test-build prerequisite and coverage scope. |
| `evals/src/runner.rs` | Replace stdout queue with optional typed readiness, bounded line assembly and continuous log draining. |
| `evals/tests/runner.rs`, `evals/tests/common/mod.rs` and runner unit tests | Deterministic high-output/cleanup helpers and private bounded-state assertions. |
| `evals/src/mcp.rs`, `evals/tests/fixture.rs` | Private decoding seams, refusal tests, exact real-server shape assertions. |
| `evals/tests/boundary.rs` | Reuse subset predicate for name/version/source negative controls and retain existing workspace/TLS checks. |
| `evals/tests/proof.rs`, scoring/report suites | Existing full acceptance controls; extend only where a concrete regression demands it. |
| `evals/README.md`, `evals/Cargo.toml` | Accepted authority, active gates, canonical fixture path, accurate remaining limits and stale comment correction. |
| `specs/agent-eval-harness/{spec,plan,tasks}.md`, `specs/eval-measurement-repairs/{spec,plan,tasks}.md` | Reconcile current proposal/gate/signing guidance while preserving dated provenance. |
| `docs/decisions/0055-agent-eval-harness.md` | Only if needed: dated factual completion addendum after evidence exists; historical body and accepted status unchanged. |

Do not edit root source/manifests/lock/deny settings. Existing recipe bundles
inherit the canonical verifier; check that inheritance instead of copying
commands into each recipe. No new helper or shim is planned. If substantial
shared logic proves necessary, return to justify a pinned helper and its
transitive resource/completeness/tamper proofs before expanding the design.

## Implementation approach and contracts

### Gate sequence and candidate executable

Use these three checks in their existing owners:

```bash
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --all-targets --locked -- -D warnings
SWARMAIL_BIN="$candidate_bin" cargo test --manifest-path evals/Cargo.toml --locked
```

Resolve `candidate_bin` from a successful
`cargo test --workspace --locked --message-format=json-render-diagnostics`.
Capture the stream to a temporary artifact log while retaining useful console
output; any tee pipeline needs `pipefail`. Use existing jq and Cargo metadata
with locked resolution to bind the root package to its canonical manifest.
Filter `compiler-artifact`, root package id, target name `swarmail`, kind `bin`,
`profile.test == false` and a string executable. Deduplicate identical
messages but require exactly one distinct path. Canonicalize to an absolute
executable file and reject zero/multiple matches. Do not change the target
directory, consult PATH or trust an inherited `SWARMAIL_BIN`.

In CI group dependent root/eval tests in one checked shell block. In verify,
retain separate diagnostic labels but pass a safely quoted absolute path
explicitly to the eval command, or group dependent commands in one checked
invocation with distinct diagnostics. Never interpolate unquoted paths or use
JSON escaping as shell escaping. Clean temporary capture data on failure.
All failures, including offline cache misses, refuse; there is no install or
skip fallback. Standalone manual runner fallback remains unchanged by this
CI/verify prerequisite.

Keep algorithms directly in CI and the pinned seat. Actual dispatch uses the
pinned bytes, canonical resource checks, recorded engine identity and shim
digests. Extend the current resource completeness assertion if an existing
required proof file is missing from it. No candidate `scripts/eval-gate.sh`
may select the verification implementation.

### Readiness data model

Replace `Proc.lines` with optional one-shot receiver of
`(SocketAddr, SocketAddr)`; supply the corresponding existing parser at server
spawn. Agent mode has no parser/receiver. The stdout task owns parser state
and one sender, sends the first accepted pair synchronously, then drops both.
Stderr always drains to the existing shared log. Sender failure/receiver drop
never stops draining. Closed sender before readiness maps to existing harness
startup failure; the existing startup deadline remains authoritative.

Accumulate only a complete startup line, up to 8192 bytes including its
newline. On overflow discard that line through the next newline, retaining
only the discard flag, and resume at the following line. Never parse an
8192-byte continuation as a new line. EOF without a complete accepted line
and read errors before delivery refuse readiness. After readiness stop
assembling lines entirely. Read chunks remain finite; `Log` keeps at most
65536 bytes after each append, with finite temporary chunk overhead. Document
these separate bounds instead of claiming a precise total heap bound.

Keep process groups, child handles, health and SMTP probes, startup/agent/I/O/
shutdown deadlines and cleanup precedence. No serialized result or public
command changes. Preserve read-error diagnostics where feasible without a
new public error taxonomy.

### Boundary tests and proof ownership

Refactor lock subset computation minimally to accept lock strings. Mutate
name, version and source separately in disposable inputs and assert the same
predicate used for real locks refuses. Keep current self-package exception,
workspace/publication and TLS checks. Do not build a second lock parser.

In fixture tests inspect raw real initialize/tools responses for the exact
server shape, and exercise actual JSON-RPC/tool error paths. In private MCP
decoder unit tests prove every AC4 malformed control and preserve harmless
extra-field/block tolerance. Do not manufacture a mock server or migrate the
client to structured results/SSE/new protocol.

For readiness use deterministic helper children that write explicit markers
and await barriers under real deadlines. Assert parser/log bounds directly
and observe child progress while the server owner retains `Proc` after
announcement. Test receiver drop, oversized suffix forgery, EOF/read error,
agent noise and descendants retaining pipes. Unit helpers cover private state;
real process integration covers draining and terminate/kill/reap. Restore
controls after collecting red evidence against an isolated baseline copy.

Add an explicit `eval-dispatch` proof mode to the existing pinned delivery
proof script, invoked by the implement/verify evidence workflow outside the
verifier's default nested proof call. Keep its default mode nonrecursive.
Use disposable candidate copies, the unchanged actual pinned verifier and
recorded engine, and prompt/result files. Mutate only eval formatting, lint
and tests in independent controls so preceding checks pass and the named
check refuses. A restored candidate must pass full dispatch. Assert result
JSON as well as process status: the verifier may exit zero while writing
`result: fail`. Run custom target/decoy env and artifact-selection controls,
and preserve existing shim/identity proofs. No fake cargo/engine, edited gate
algorithm, skip flag or command-string assertion substitutes for this proof.
Proof cost is accepted; logs stay run-local. Root tests can assert wiring
without launching this outer dispatch proof recursively.

### Documentation reconciliation

Update current prose only after wiring exists. Cite existing operator
acceptance, active gates and provenance. Link the sole inventory rather than
creating another authoritative list. Preserve historical dated entries and
real deferred capabilities; explicitly supersede stale signing instructions
with 0056. If explaining 0055's historical deferral requires a change, append
a dated completion note grounded in actual gate evidence. Add the canonical
#149 fixture path/subcommand to README, search for bad local references and
record the precise external issue-text handoff if still unavailable. Do not
mutate external issues or decide policy routing.

## Quickstart and validation

From this checkout, run eval fmt/lint, then the root test artifact capture
above and full eval tests with the selected absolute executable. The full
suite includes boundary, fixture, runner, scoring/report and 16-oracle/
16-null/overlapping-isolation proofs. Run targeted tests during development;
collect red/restored-green evidence in isolated disposable candidates.

Run every command in [the house-rule inventory](../../docs/house-rules.md#gates--part-of-every-change),
including the coverage selftest, publish/seal proofs, license and fail-closed
native-TLS checks, plus design selftest, delivery tamper proof and the new
outer eval dispatch proof. Use the digest-matched local Brokkr for compile
and dispatch; compile inherited recipes affected by canonical resource edits.
Record commands, candidate SHA, selected executable, exits and decisive logs.
Do not claim CI-owned Docker/chart/advisory/tag/port-80 proofs locally.
Failures and unavailable tools remain outstanding; no installation in verify.

This design commit stages only `specs/eval-gates/{spec,plan,tasks}.md`.
Validate the committed HEAD with `bundles/self/scripts/design-check.sh check`
and leave it last. Implementation later edits older specs and runtime owners;
those edits do not belong in the chief commit. Run-local positions, gate logs
and result JSON remain ignored.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Green eval uses stale or test-harness binary. | Canonical package artifact selection, root verdict first, absolute executable check, decoy/custom-target controls; refuse ambiguity. |
| Captured output hides root failure or breaks on non-JSON tests. | Preserve Cargo exit with pipefail; parse only valid artifact records; retain human diagnostics. |
| Pinned verify loses environment or executes candidate logic. | Explicit quoted path propagation, pinned script ownership, actual dispatch proofs and existing pin/shim controls. |
| Added proof recursively launches itself. | Explicit outer eval-dispatch mode, nonrecursive default proof, real fixtures and result JSON assertions. |
| Readiness fix stalls children or loses announcement. | One-shot parsed pair, no awaited send, whole-line bound, continuous draining and deterministic retained-server proof. |
| Oversized suffix looks like readiness. | Discard through newline, exact-limit/oversize/forged-suffix tests. |
| Root green is misrepresented as complete eval coverage. | Standalone boundary remains explicit; both suites required; no expansion of root denominator. |
| Offline cache or platform dependencies are missing. | Fail visibly and record blocker; no install/skip fallback; both OS CI jobs run. |
| MCP assertions accidentally tighten public compatibility. | Exact real-server shape tests separate from existing decoder refusals and tolerated extras. |
| Stale documentation fabricates acceptance or external edits. | Cite recorded 2026-10-19 status, dated addendum only, explicit #149 handoff and no issue mutation. |
| Concurrent eval work is overwritten. | Work only here; record baseline, inspect overlaps, integrate deliberately and rerun affected proofs. |
| Gates become more expensive. | Reuse jobs, one root test build and existing test owners; keep finite deadlines and full required evidence. |
