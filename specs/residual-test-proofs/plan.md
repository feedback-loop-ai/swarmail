# Plan: residual test proofs

## Technical context

One Rust 2024 crate, existing Tokio networking/time support, real HTTP/SMTP
integration servers, and a `Mutex<BTreeMap<String, u64>>` cumulative registry.
No dependency, feature, migration or production API change. Linux and macOS
are the refusal proof platforms. Start from inspected `db0b517`, not the
pre-repair integrated `34eb99f` used to audit the retained implementation.

No plan template is present under `.specify` or `bundles`; this plan provides
the required technical context, constitution check, structure, research,
data model, contracts, quickstart and risks directly. No additional document
set or generic proof framework is needed.

## Constitution check

Preserve frozen 0001–0003 and ingest extraction (0005), real isolated protocol
observations (0004/0010), independent webhook delivery (0041/0048), Host/Origin
fixtures (0043) and one cumulative snapshot (0047 §2). Keep branch-stable
assertions inline and measured under 0020; no exclusions, feature seams or
coverage-floor changes. Follow pinned proof ownership under 0050, existing
`engine` triage/protected review under 0057, and unsigned author commits under
0056. No semantic amendment and no new decision record.

## Project structure and files

| File | Evidence-backed role |
|---|---|
| `Dockerfile` | Preserve current AC52.2 comment and digest; compare bytes with `34eb99f`. |
| `tests/common/mod.rs` | Preserve corrected helper prose and the removal of only `wait_for_metric`; no body/signature change. |
| `tests/webhook.rs` | Preserve the held non-listening socket and bounded refusal probe in `refused_connection_never_blocks_the_smtp_path`. |
| `src/api.rs` | Preserve the inline `cumulative_scrape_uses_one_registry_snapshot` assertion; production remains unchanged. |
| `specs/residual-test-proofs/{spec,plan,tasks}.md` | Exactly the three artifacts revised and committed by the chief. |

These implementation roles are not instructions to manufacture edits.
Production `cumulative_rows`, `render_rows`, store, SMTP, dispatcher, validation,
Cargo, gates and decisions are read-only context except the explicitly
uncommitted, restored mutation proof in a later authorized implementation phase.

## Research and resolved choices

Both current panel positions and intake agree that all five repairs appear
present at `db0b517`. The inspected comparison with `34eb99f` changes only
helper prose/deletion, the refusal fixture and inline snapshot test. Dockerfile
has no diff. Exact reference search found only spec prose for the removed
wrapper. These observations are not execution receipts.

Reuse the owning design rather than create a competing slug. Preserve the
simple implementations and demand robustness's discriminating evidence. The
current panel has no substantive conflict; remove the prior design's stale
attribution of tolerant normalization to the current simplicity position.
Accept strict source-fixture maintenance and one held socket descriptor.
Platform behavior and mutation discrimination remain empirical obligations.

## Implementation and proof approach

1. Record candidate identity and clean starting state. Inspect the retained
   diff against `34eb99f` and compare Dockerfile bytes. Repeat exact reference
   search including relevant hidden sources; exclude `.git`, `target`,
   `.brokkr` and `.forge`. Classify documentary hits separately. Preserve
   existing helper documentation and live pollers. No deletion is needed at
   the inspected baseline; never restore the dead wrapper merely to delete it.
2. Confirm the existing refusal fixture uses `TcpSocket::new_v4()`,
   `set_reuseaddr(false)`, `bind(127.0.0.1:0)` and `local_addr()`.
   It neither listens nor connects the reservation socket. A separate
   `TcpStream::connect` under `HELPER_DEADLINE` must produce
   `ConnectionRefused`. Keep ownership through PUT, baseline, SMTP, first
   count, exhaustion observation and final count. Explicit final `drop` makes
   lifetime reviewable. Run the same test on Linux and macOS; no skip,
   listener or retry fallback closes a failure.
3. Confirm the existing source assertion normalizes CRLF, restricts source
   to the prefix before `#[cfg(test)] mod tests`, and requires the full
   `cumulative_rows` signature plus following `render_rows` boundary. It
   compares the complete function with outer trim only. Preserve the exact
   output assertion before the source assertion and this reviewed body:

   ```rust
   fn cumulative_rows(registry: &Registry, name: &str) -> String {
       let units = registry
           .lock()
           .expect("a readiness registry mutex cannot poison");
       render_rows(&units, name, "counter", units.values().sum())
   }
   ```

   No token counter, parser, callback, new harness or runtime instrumentation.
   Extraction cannot use its own embedded expected literal as production.
4. Run the focused snapshot test green, confirming exactly one named test ran.
   Save the exact original `src/api.rs` bytes in ignored run evidence before
   temporarily replacing only production `cumulative_rows`. The mutant sums
   under a scoped guard, releases it, takes another guard and renders with the
   saved total. Leave the expected literal unchanged. Run the identical test:
   it must compile and reach the source assertion failure after its exact
   output assertion passes. Capture that diagnostic and exit status. Arrange
   restoration even on proof failure; restore saved bytes, verify byte equality
   and empty production diff, then rerun green. Do not stage mutants or accept
   compilation failure as proof. Execute this only in the later phase authorized
   to mutate implementation, not during chief synthesis.
5. Run existing affected consumer suites against real fixtures. Record command,
   candidate identity, platform, exit status and diagnostic in ignored run-local
   evidence. Inspect scope again after proof restoration. If all criteria hold,
   no additional implementation commit is required. If a criterion fails,
   make only an evidence-backed repair within its named region; escalate before
   introducing runtime changes, live caller migration, new seams/harnesses,
   incompatible platform handling, dependencies or unrelated files.
6. Run the complete house gate inventory with pinned ownership, then hand
   evidence to independent verify/review. Do not infer a pass from compilation,
   earlier inspection, unchecked tasks or an unavailable platform. Report a
   blocked proof accurately; do not widen this patch to repair unrelated gates.

## Data model and contracts

No persistent data, schema, metric family or public contract changes. The
reservation is one test-local socket whose lifetime is setup → probe → PUT →
baseline → SMTP → first count → exhaustion observation → final count → drop.
The deadline bounds failure; it is not a latency success assertion. One fresh
server isolates the default inbox. Both stored counts remain exactly one and
exhaustion is one new event over the pre-mail baseline.

The snapshot test owns its private registry. Exact output includes disjoint
LF overflow, while the complete-body source assertion pins the reviewed
single-guard implementation. It does not count arbitrary runtime acquisitions
or prove uninspected callees. Queryability before observing exhaustion does
not establish SMTP-versus-retry temporal ordering. Best-effort webhook loss,
storage ordering and acceptance semantics remain as ruled.

## Quickstart and verification

```bash
cargo test --locked --lib api::tests::cumulative_scrape_uses_one_registry_snapshot -- --exact
# Repeat with the compiling split-lock mutant, require source-assertion failure,
# restore exact saved bytes, then repeat the same test green.
cargo test --locked --test webhook refused_connection_never_blocks_the_smtp_path -- --exact
cargo test --locked --test api --test lifecycle --test binary --test webhook --test mcp
```

Confirm each focused command executes its named test, not zero filtered tests.
Execute refusal on both Linux and macOS. Existing API tests cover Host/Origin;
lifecycle and binary cover helper deadlines/HTTP reliability; webhook and MCP
cover counter/labeled-row consumers. No new tests are needed for moved prose
or an unused-wrapper deletion.

Then execute [the sole house gate inventory](../../docs/house-rules.md#gates--part-of-every-change)
in full, with the candidate-local digest-matched `.forge/bin/brokkr` and pinned
proof scripts/checked shims as required. Include exact coverage and selftest,
publish/seal proofs and fail-closed native-tls absence; unrelated Cargo failure
is not evidence of absence. CI-only chart/history/selftest, Docker smoke,
RustSec, tag release-rate and real port-80 evidence remain distinct per the
house rules. Record unavailable proofs as outstanding, never local passes.

Chief validation is limited to artifact/link/diff checks and the committed
structure check; implementation mutation and delivery gates remain later work.
Keep the chief design commit last and write the ignored result through workspace
hands. The sequence's final pinned check owns advancement.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Existing code is mistaken for completed acceptance. | Keep tasks unchecked until actual receipts; permit zero extra edits but require all proofs. |
| Listener or early release changes the failure mode. | Bound non-listening socket, reuse disabled, bounded real refusal, explicit final drop; one held descriptor. |
| Linux results are presented as macOS proof. | Execute both; missing platform evidence stays outstanding, no fallback or skip. |
| Idle output hides separate snapshots. | Require compiling split-lock failure at the source assertion and restored green. |
| Extraction finds its expected fixture or silently loses anchors. | Production prefix, mandatory full signature/next declaration, complete-body comparison. |
| Benign formatting/refactoring fails the guard. | Accept maintenance cost; review literal updates and rerun discrimination. No claim of general runtime lock counting. |
| Inline changes evade or break coverage. | Keep straight-line assertions measured; full exact gate/selftest, no exclusion or gratuitous harness move. |
| Helper cleanup changes security or event observations. | Preserve request bytes, deadlines, pre-trigger baselines and labeled pollers; execute real consumers. |
| Temporary mutant leaks into delivery. | Save/restore exact bytes even on failure, verify byte equality and scope before staging or gates. |
| Unrelated failures provoke scope growth. | Record evidence and return for scope escalation; no runtime, TLS, dependency or gate repair under this design. |
