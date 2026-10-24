# Checked usize proof — issue #180 continuation

Date: 2026-10-24.
Status: design draft; implementation and delivery proofs remain outstanding.
Scope: Epic #192, issue #180, run epic-192-follow-up-for-issue-180-b92c6fdb.

## Feature and rationale

Make the existing checked MCP integer refusal measurable on the ordinary
64-bit host while retaining its actual target-width behavior. This is a
behavior-preserving refactor and a repair of test fixtures, with no new
public contract, frozen guarantee or ruling. No new decision is proposed.

The existing [RPC edge specification](../rpc-edge-contracts/spec.md) AC1–AC10
continues to govern all six issue #180 repairs. This follow-up narrows the
implementation to the remaining integer proof gap; it neither reopens
satisfied criteria nor closes outstanding evidence by assertion. Authority:
accepted [0029](../../docs/decisions/0029-supplied-values-are-validated-wherever-they-reach-a-wire.md),
[0044](../../docs/decisions/0044-compat-validation.md),
amended [0047](../../docs/decisions/0047-storage-rpc.md),
[0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md),
[0035](../../docs/decisions/0035-coverage-gate-refuses-by-effect.md) and
[0050](../../docs/decisions/0050-delivery-gates.md).
Existing [0062](../../docs/decisions/0062-rpc-edge-contracts.md) stays proposed;
this citation neither declares a new decision nor accepts that record.

## Evidence and synthesis

Both run-local panel positions, simplicity.md and robustness.md under
.brokkr/state/design/positions/, and intake
.brokkr/state/tasks/epic-192-follow-up-for-issue-180-b92c6fdb.md were read.
At candidate dd76dfe54e31a7ff4fa8e2d4b90f2377968056a8, retained verify receipt
.forge/results/f1142bcc-07b1-49fe-8d8c-47ddc00a2e51.json refuses coverage.
Its target/coverage/run.T2os3A/coverage-summary.json reports 13078/13079
lines, 766/766 branches and 1444/1445 functions. src/mcp.rs:285 is the
conversion-error closure: every u64 fits usize on this host. These are
historical measurements, not final-candidate passes.

Both actual-32-bit witnesses exist, but no successful runtime receipt was
retained. The earlier implementation records missing musl-gcc and multilib
headers. Chief inspection also found both witnesses use
`u64::from(usize::MAX)`, which does not compile: an isolated rustc metadata
probe against installed i686-unknown-linux-musl produced E0277.
`u64::try_from(usize::MAX).unwrap()` compiled in the same probe.
Logs are .forge/design-usize-proof/fixture-probe.log and fixture-green.log.
These probes establish fixture compilation only, never runtime behavior.

| Position or tension | Ruling and cost |
|---|---|
| Both panels propose one private concrete conversion-result mapper. | Adopt. Production still calls usize::try_from unconditionally and passes its Result to that mapper. Accept one extra measured function and local indirection so the real refusal construction can execute on the host. |
| Simplicity limits edits to src/mcp.rs and corrects the paging success fixture to usize::MAX. | Adopt the success-fixture correction; reject the absolute one-file limit because both cfg-32 witnesses contain the confirmed E0277 fixture error. Allow the same checked fixture conversion in tests/mcp.rs. Cost: one additional existing test file changes; no new harness or production owner. |
| Robustness requires actual 32-bit execution and a discriminating unchecked-cast mutation; simplicity favors minimal proof. | Require both existing 32-bit witnesses, the corrected paging test, and the narrow mutation/restoration check. Cost: runner/toolchain availability and extra executions can keep delivery open after host coverage passes. A host mapper test cannot catch an unchecked cast in its caller. |
| Robustness requests multi-field mapper assertions; simplicity avoids a framework. | Exercise success and failure with at least offset and count in the existing inline module. Cost: a few measured assertions, without generic adapters, injectable converters or new types. |
| Robustness audits broad failure boundaries; simplicity excludes unrelated implementation. | Keep those boundaries as preservation constraints; reject new storage, SMTP, webhook, TLS or resource-policy work. Cost: existing best-effort delivery and resource limits remain; this repair provides no stronger guarantees. |
| Both panels retain full review despite a small patch. | Adopt intake's engine classification under 0057. Four independent review lenses remain mandatory; patch size does not authorize a reduced lane. |

There is no genuine disagreement between the panels over conversion semantics.
Reject extracting only an error-string helper (it leaves the cold closure),
eager error allocation on every successful parse, unchecked casts, clamping,
a universal u32 ceiling, fake widths and production cfg splits. Reject coverage
exclusions, test relocation to evade measurement, report editing, merged-target
counting and gate/floor changes. None supplies the required truthful proof.

## Acceptance Criteria

1. **AC1 — Production mapping is executable.** The numeric arm of arg_usize
   passes the real `usize::try_from(n)` result to one private concrete mapper,
   whose Ok arm returns Ok(Some(n)) and Err arm returns
   `refusal(key, "a non-negative integer that fits usize")`.
   Both arms execute in measured inline host tests. A real TryFromIntError
   obtained from `u8::try_from(256_u16).unwrap_err()` proves mapping only.
   Assert the exact -32602 tuple for offset and count, with no field hardcoding.
   No separate unreachable conversion-error closure remains in arg_usize.
2. **AC2 — Caller behavior is preserved.** Omitted arguments keep defaults;
   null, negative, fractional and other wrong-shaped values retain named
   non-negative-integer refusals. Zero and usize::MAX stage successfully.
   Search limit/offset, latest count and wait count keep their current defaults,
   page caps, uncapped representable offsets, latest-zero refusal and
   wait-zero normalization. Invalid inputs still refuse before scans or
   waiter registration. Existing host unit and real HTTP controls remain green.
3. **AC3 — Fixtures describe the executing target.** The successful empty
   search page uses usize::MAX instead of u64::MAX and still preserves total.
   Both cfg-32 tests construct max through
   `u64::try_from(usize::MAX).unwrap()`, retain max+1 as u64, and keep their
   existing acceptance and exact overflow-refusal assertions. This permits
   32-bit compilation without narrowing any production domain.
4. **AC4 — Actual 32-bit proof remains mandatory.** Execute
   usize_staging_accepts_max_and_refuses_overflow_on_32_bit,
   mcp_search_offset_refuses_values_beyond_32_bit_usize and
   search_pages_with_offset_and_keeps_the_total on an actual 32-bit binary.
   Accept 4_294_967_295 at staging/search offset; refuse 4_294_967_296 with
   -32602 and the exact offset-naming sentence. Record source identity, target,
   runner, exact --locked commands, exits and nonzero executed test counts.
   Replacing the production checked conversion with an unchecked cast must
   make the overflow witness fail behaviorally; restore exact bytes and rerun
   green. Missing tools, compile-only probes and zero-test runs do not pass.
5. **AC5 — Exact host coverage and gates.** Reproduce the unchanged-source
   refusal, then require covered == count with nonzero counts for lines,
   branches and functions on the repaired candidate. Run every gate in
   [house rules](../../docs/house-rules.md#gates--part-of-every-change),
   coverage selftest, pinned shim checks, verify-owned proofs and fail-closed
   native-tls absence. No dependency, license, gate, policy or recipe changes.
6. **AC6 — Existing contracts stay intact.** Preserve prior AC1–AC10,
   amended 0047's invalid-ID -32600/null refusal before effects and valid
   scalar/null IDs, effectful silent notifications, batch ordering/admission,
   stdio framing, warning route and original Host bytes. Preserve synchronous
   accept==stored, subscribe-before-check, oldest-only pruning and extraction
   at ingest. Store, persistence, SMTP, webhook and stdio production owners
   remain unchanged by this repair.
7. **AC7 — Evidence and review stay honest.** The engine-selected review
   retains Qwen Max correctness, Claude Opus security, official DeepSeek Flash
   guarantees (never Pro) and DashScope GLM 5.3 tests, with no fallback and
   protected security hold. Distinguish local coverage from actual 32-bit,
   macOS stderr, privileged port-80 and CI chart/Docker/RustSec/tag-release
   evidence. Missing receipts stay open; neither this draft nor host coverage
   alone closes issue #180 or Epic #192.

## Preserved failure boundaries

Validation remains before admission, while Store subscription remains before
its first scan. An SMTP 250 follows synchronous queryable insertion; persistent
commit-before-reply uncertainty may cause sender retries, not exactly-once
delivery. Pruning remains oldest by acceptance order inside the mutation
boundary. Webhooks remain bounded best-effort post-commit fan-out, with no
durable outbox. Hostile DATA handling, ingest-time extraction, rustls checks,
generated TLS fixtures, isolated test stores/inboxes and existing concurrency
bounds remain intact. No global memory ceiling, trickle-session bound or
crash-proof notification delivery is added.

## Design delivery boundary

Commit exactly this directory's spec.md, plan.md and tasks.md, unsigned.
Run-local positions and receipts remain ignored. The design commit stays HEAD
for the deterministic check; no push, merge, tag or publication. Only the
sequence's check owns its validation verdict. This seat returns drafted.
