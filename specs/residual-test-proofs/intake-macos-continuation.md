# Intake: finish the macOS SIGTERM fixture repair

Status: resolved framing; implementation, final verify/review and macOS CI remain outstanding.
Run: `epic-192-issue-191-finish-the-ma-fe6a4a27`.
Date: 2026-10-25. Intake baseline: `94b8ae15f0b3515d5860266f0988a2a4b75f15b0`.

## Goal

Correct the owning spec and the SIGTERM delivery test's Linux-only reuse-enabled bind expectation while retaining the connected-client target, exact connect refusal and existing shutdown proof.

## Authority and bounded inspection

Read `.forge/epic192/issue191/README.md` and `operator-rulings.md` first;
all four entries in that directory's `SHA256SUMS` verified when checked from
that directory. Read the full `review-78a81fc6.md`: independent finding 1 is
the HIGH spec defect and finding 2 is the related macOS ownership limit.
The feature request directs their narrow correction; the operator's dated
#191 ruling assigns actual macOS execution to CI. No further acceptance is
inferred and no external evidence is needed for framing.

Read the house charter, decision index, relevant shutdown/isolation rulings
0004, 0018 and 0024, regression placement 0020, final-ledger contracts
0041 sections 5–6 and 0048 section 6, pinned-proof ruling 0050 and unsigned
commit policy 0056. Spot-checked the named function, owning spec/plan/tasks,
and local commits `f5b29d4` and `94b8ae1`. The code sets reuse false on the
connected client, retains its accepted peer and exact bounded refusal probe,
then requires contender binds with reuse false AND true to fail. Spec ruling
2 / AC4, plan step 5 and task T3 explicitly mandate that faulty expectation.
Do not re-derive the supplied platform review or redesign the fixture.

This continuation supersedes earlier intake/design claims of universal
exclusive ownership only where the review identifies them. Those Linux
receipts remain historical evidence, not macOS proof or proof of this repair.
Reconcile active contradictory wording with a dated correction; preserve
historical evidence and do not rewrite accepted decisions.

## Checkable acceptance criteria

1. **Portable contender contract.** In
   `tests/binary.rs::sigterm_with_a_delivery_in_flight_prints_the_final_counts`,
   retain the final reuse-disabled contender bind and exact `AddrInUse`
   assertion. Either remove only the reuse-enabled contender assertion, or
   explicitly express its expected result per supported platform: Linux
   `AddrInUse`, macOS successful bind. An unconditional reuse-enabled
   `expect_err`/`AddrInUse` requirement is forbidden. Never listen on a
   contender. Source diff and the focused real-server test prove the local
   branch; macOS execution belongs to CI.
2. **Honest spec and fixture limits.** Amend spec ruling 2 / AC4, plan step 5
   and tasks T3, plus directly conflicting active wording/comments. They must
   state that the held connected pair proves lifetime and exclusion against
   reuse-disabled binders; it does not establish exclusion of reuse-enabled
   listeners on macOS, including parallel Tokio listeners. Do not claim the
   macOS port-reuse race is fully closed. All three design artifacts must
   agree with the chosen contender check; a source/text comparison fails
   this criterion if any active requirement still demands both binds fail
   on macOS or asserts universal exclusion. Historical receipts must be
   explicitly distinguished from current proof.
3. **Preserve the experiment.** Keep the explicitly bound reuse-disabled
   connected client and accepted peer alive through all final assertions,
   dropping the pair last. Keep setup connect/accept and the real
   `TcpStream::connect` probe bounded by `HELPER_DEADLINE`; that probe must
   still require exactly `ConnectionRefused` at the same client address used
   by the webhook. Preserve existing stalled-request/exhaustion observations,
   SIGTERM, EOF/reap, successful exit, exactly one final message with
   delivered/dropped/lagged/exhausted/aborted totals 0/0/0/1/1, exactly one
   exhaustion warning with substantive transport cause, message formatting
   and absence of hunter2/hunter3/marker-secret on stdout. No timeout-as-pass,
   broad error matching, sleeps, probabilistic retries or new ignores.
4. **Narrow diff.** Relative to intake baseline `94b8ae1`, code changes are
   limited to the named function's contender check and directly related
   explanatory comments. Owning spec/plan/tasks and this continuation record
   may change to correct the defect. Everything else remains byte-identical:
   no production, helper, separate webhook/snapshot regression, dependency,
   lockfile, allowlist, accepted-decision, gate or recipe-routing changes.
   Inspect `git diff 94b8ae1` to enforce this boundary.
5. **Completed local proof.** On the committed implementation candidate,
   the focused command below selects exactly one passing test; the full
   binary suite and every local house gate exit zero. Exact coverage requires
   zero missed lines, branches and functions plus its passing selftest;
   output followed by a timeout does not qualify. Retain command, platform,
   candidate commit/tree, output and exit receipts. Commit all scoped work
   unsigned before final verify/review; retain existing bundles' routing and
   protected review/security hold. Intake resolution is not feature delivery.

## Frozen guarantees and proof ownership

Adjacent to 0001 through real SMTP acceptance and to shutdown observation
under 0018/0024/0041/0048; no frozen guarantee is changed. Synchronous
accept/store (0001), subscribe-before-check (0002), oldest-only pruning (0003)
and ingest extraction (0005) remain untouched. This is test/spec correction
within accepted semantics, so no semantic ADR or decision acceptance is needed.

CI owns the actual macOS binary-test pass under the operator ruling. It also
owns the charter's chart/history proofs, Docker smoke, network RustSec audit,
privileged real port-80 exchange and release-only proof. The operator owns
sealing, live GitHub settings and publication. These are external handoffs,
not additional local acceptance blockers or claimed executions. No GitHub,
CI, journal, sibling worktree, namespace or network inspection is authorized.

## Verify commands

Run offline with cached tools; install nothing. No benchmark is required for
this test-only correction. The charter remains the single gate inventory.
The public shim digests must match; under 0050 execute pinned proof copies
against the candidate, as the verify seat does:

```bash
cargo test --locked --test binary sigterm_with_a_delivery_in_flight_prints_the_final_counts -- --exact
cargo test --locked --test binary
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

Reject a compile policy-sweep warning even with exit zero. Also run this
fail-closed dependency absence proof; arbitrary Cargo failure is not success:

```bash
python3 - <<'CHECK_NATIVE_TLS'
import subprocess
r = subprocess.run(
    ['cargo', 'tree', '--workspace', '--locked', '--offline', '--invert', 'native-tls'],
    text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
)
print(r.stdout, end='')
expected = {
    "error: package ID specification 'native-tls' did not match any packages",
    "error: package ID specification `native-tls` did not match any packages",
}
if r.returncode == 0 or not expected.intersection(r.stdout.splitlines()):
    raise SystemExit('native-tls absence was not proven')
CHECK_NATIVE_TLS
```

## Intake receipt

Evidence and engine digests verified; workspace started clean at `94b8ae1`.
No missing workspace input blocks framing. This phase changes no Rust or
owning design implementation. Any gate executions below exercise the existing
Linux fixture with the known macOS spec defect, and cannot certify the future
repair or claim a macOS pass. Final implementation must rerun all proofs.
