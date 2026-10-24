# Plan: residual test proofs

## Technical context

One Rust 2024 crate with existing Tokio networking/time support, real HTTP/SMTP
fixtures and a mutex-protected cumulative registry. No new dependencies,
features, schemas or runtime API. Inspected HEAD is `ffca01b`; compare the
three earlier draft commits against integrated rollup-a `c3d124d`.
Linux is the local proof platform; macOS is CI-owned.

No plan template exists under `.specify` or `bundles` in this candidate.
This plan supplies the required context, constitution check, structure,
research, data model, contracts, quickstart and risks directly. No separate
research or contract document set is needed for this bounded cleanup.

## Constitution check

Preserve 0001–0005, webhook delivery and final ledger (0041/0048), verbatim
Host/Origin fixtures (0043), and the one-snapshot contract (0047 §2).
Keep the existing branch-stable snapshot test inline and measured (0020).
No exclusions, denominator reduction, feature seams, dependencies or allowlist
changes. Use pinned proofs (0050), unsigned author commits (0056) and current
candidate routing (0057). No semantic amendment or decision document.

## Project structure and files

| File | Implementation role |
|---|---|
| `Dockerfile` | Inspect AC52.2 and byte equality with `c3d124d`; preserve it. |
| `tests/common/mod.rs` | Preserve corrected prose and deleted wrapper; audit unchanged live helper behavior. |
| `tests/binary.rs` | Repair only the refused-target setup/comment and ownership lifetime in `sigterm_with_a_delivery_in_flight_prints_the_final_counts`. |
| `tests/webhook.rs` | Preserve the distinct held-port, pre-trigger-baseline and count proof. |
| `src/api.rs` | Preserve inline `cumulative_scrape_uses_one_registry_snapshot`; production is unchanged except during a restored negative control. |
| `specs/residual-test-proofs/{spec,plan,tasks}.md` | The chief's only three committed artifacts. |

Store, SMTP, dispatcher, rendering, validation, Cargo, decisions, gates and
routing are context, not implementation targets. No new source files.

## Research and resolved choices

Packet checksum and candidate-local engine digest match. Both current panel
positions and intake correctly identify the still-released binary port.
The draft changed the separate webhook test instead. The wrapper deletion and
source guard are already present. Reuse this design and replace contradictory
active requirements while retaining the dated historical clarification.

Choose a local socket over a shared reservation abstraction. Choose the small
existing source fixture over runtime acquisition instrumentation. These choices
accept a little duplicated fixture code and deliberate refactoring sensitivity;
they still require actual refusal and mutation discrimination. The spec records
the adopted/rejected alternatives and their costs.

## Implementation approach

1. Record HEAD, platform and status; inspect the draft diff from `c3d124d`.
   Compare Dockerfile bytes. Search exact `wait_for_metric` references across
   relevant source/imports, distinguishing documentation and excluded run/build
   evidence. Confirm helper paragraph ownership and unchanged bytes/deadlines.
   Do not recreate the wrapper or change already-correct implementation.
2. In the named binary test replace the temporary listener with
   `tokio::net::TcpSocket::new_v4()`, `set_reuseaddr(false)`, and binding to
   `127.0.0.1:0` without listening. Save `local_addr()` as the refused target.
   A separate `TcpStream::connect` under `common::HELPER_DEADLINE` must yield
   `ErrorKind::ConnectionRefused`. Keep the reservation variable in the test
   and explicitly drop it after all cause/redaction assertions. RAII releases
   it on failure. Preserve the stalled sink, child lifecycle, zero baseline,
   exact ledger and secrets checks. Add no retry, sleep, skip or fallback.
3. Preserve the webhook fixture's existing held socket/probe and pre-trigger
   baseline. Its final stored count and explicit drop remain. Run both named
   tests on Linux. Their macOS execution belongs to CI when the PR opens.
4. Review the `/metrics` call to `cumulative_rows` and its `render_rows`
   callee. Preserve the snapshot test's output-first assertion and complete
   function comparison with mandatory production-only anchors and CRLF/outer
   trim only. Run exactly the named library test and confirm one test ran.
5. During implementation, save the original `src/api.rs` bytes in ignored
   evidence. Arrange restoration on failure before changing anything. Replace
   only production `cumulative_rows` with a compiling scoped-lock total
   calculation, guard release, second lock and render with saved total.
   Leave the expected literal untouched. Run the identical named test: output
   must pass, then the structural assertion must fail with its named message.
   Capture exit and diagnostic; compilation errors or zero tests refuse proof.
   Restore exact bytes even on failure, compare bytes, check no production
   diff, and rerun green. Never stage the mutant. No implementation mutation
   occurs in this chief design phase.
6. Run affected real-server consumers and inspect final scope. Record receipts
   (candidate, command, platform, exit, diagnostic) in ignored evidence. Any
   newly necessary wider change returns for scope evaluation; it is not grounds
   to introduce runtime instrumentation or repair unrelated gates here.
7. Run every local house gate on the repaired candidate and hand the receipts
   to independent verify/review. Preserve current candidate recipe routing and
   hard security hold. Do not infer completion from inspection or old receipts.

## Data model and contracts

No stored data, schema or metric family changes. The binary test owns one
reservation across setup → refusal probe → configuration → SMTP → stalled
request/exhaustion observations → SIGTERM → EOF/reap → ledger/cause/redaction
assertions → drop. Fresh-child metrics justify the existing zero baseline.

The webhook test separately owns setup → probe → PUT → baseline → SMTP →
count one → new exhaustion → count one → drop. Observation order proves stored
mail at those moments, not temporal ordering of SMTP receipt and background
exhaustion. Delivery remains best-effort and independent of stored-mail truth.

The snapshot test owns a private registry and verifies exact total/rows with
LF overflow. Its source guard pins a reviewed small function, not arbitrary
callee acquisitions or every possible endpoint change. Deadlines bound fixture
failure; no latency success contract is added.

## Quickstart and verification

```bash
cargo test --locked --test binary sigterm_with_a_delivery_in_flight_prints_the_final_counts -- --exact
cargo test --locked --test webhook refused_connection_never_blocks_the_smtp_path -- --exact
cargo test --locked --lib api::tests::cumulative_scrape_uses_one_registry_snapshot -- --exact
# Repeat the snapshot command for the compiling negative control and exact restoration.
cargo test --locked --test api --test lifecycle --test binary --test webhook --test mcp --test smtp_edges
```

Confirm focused commands run one named test. The consumer suites exercise raw
Host/Origin inputs, deadlines, counter baselines and labeled gauge observations.
No new test is needed for moved prose or an unused function's deletion.

Execute [the sole house gate inventory](../../docs/house-rules.md#gates--part-of-every-change)
in full. Use `.forge/bin/brokkr compile --bundle bundles/self` after checking
its SHA-256 against the spec and tooling manifest. Verify pinned shim ownership;
independent verify runs the pinned copies. Coverage must exit successfully with
zero missed lines, branches and functions and pass its selftest. Publish-guard,
seal-branch and licenses are mandatory. For native-tls, successful `cargo tree
-i native-tls` means refusal; an error passes only when it explicitly reports
no matching package. Other errors cannot prove absence.

CI owns macOS, chart history/selftest, Docker smoke, RustSec, tag release-rate
and real port-80 proof as specified by the house inventory. These are CI-owned,
not pending local tasks or claimed passes. Design-phase gate receipts concern
the unrepaired candidate and never replace final implementation/verify runs.
The chief checks links, scoped diff and committed structure and leaves its
design commit last; the sequence's final check owns advancement.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| A separate webhook fix is mistaken for binary acceptance. | AC4 names the actual binary test and requires its continuous ownership. |
| A listener or early release changes the failure mode. | Bound non-listening socket, reuse disabled, exact bounded refusal, explicit final drop. |
| Platform assumptions are reported as measured facts. | Linux local execution; macOS CI ownership, no skip/fallback or fabricated pass. |
| Idle output hides split locks. | Compiling split-lock negative control must fail the structural assertion after output passes. |
| Source extraction finds its own expected literal. | Exclude the test suffix and require full production signature/next-declaration anchors. |
| Benign refactoring fails the literal. | Accept maintenance burden; review and re-prove intentional updates. |
| Mutant survives a failed proof. | Save exact bytes, install cleanup before mutation, verify restoration and rerun green. |
| Helper cleanup alters boundary fixtures. | Preserve bytes, signatures, deadlines, baselines and live pollers; run real consumers. |
| Coverage or a timed-out gate is misreported. | Require zero missed counts plus successful gate exit and selftest; preserve measured placement. |
| Old draft claims substitute for current evidence. | Keep provenance labeled historical; use current receipts and unchecked implementation tasks. |
