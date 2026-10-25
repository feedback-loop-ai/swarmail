# Intake: make the issue #191 SIGTERM delivery test portable

Status: resolved framing; implementation and final verification remain outstanding.
Run: `epic-192-issue-191-make-the-sigt-78a81fc6`.
Date: 2026-10-25.
Candidate before intake: `90eb2a0333a751576c94d82474e4410040d68d43`.

## Goal

Repair `tests/binary.rs::sigterm_with_a_delivery_in_flight_prints_the_final_counts` so Linux and macOS can exercise its existing shutdown ledger proof with an exclusively owned target whose delivery fails during connect.

## Evidence and authority

Read `.forge/epic192/issue191/README.md` and `operator-rulings.md` first;
`sha256sum -c SHA256SUMS` in that directory verified both and
`macos-ci-failure.md`. The latter records the deterministic macOS
`TimedOut` versus `ConnectionRefused` failure and the Linux passes.
The 2026-10-25 operator ruling authorizes this portability repair and assigns
macOS proof to CI. No further decision acceptance is inferred.

Spot-checked the named test and commit `320f514`, which introduced its
held `TcpSocket`, bounded connect probe and exact `ConnectionRefused`
assertion. The reservation currently stays alive through the final assertions.
The test already observes the stalled sink, exhaustion, SIGTERM, EOF and reap,
then asserts the final ledger and credential-safe cause diagnostic.
The existing owning spec's AC4 hard-codes the defective fixture; design must
reconcile only the statements this repair supersedes. Prior #191 fixes,
including the separately reviewed snapshot discriminator, remain preserved.

Read the house charter, decision index and relevant accepted decisions:
0004 (real protocol and test isolation), 0018 (bounded shutdown observation),
0020 (deterministic regression placement), 0041 sections 5–6 (final ledger),
0048 section 6 (message, transport cause and redaction), 0050 (pinned executed
proofs) and 0056 (unsigned commits). This is a test portability repair within
accepted semantics; no new semantic decision is required.

## Acceptance criteria

1. The named binary test retains exclusive ownership of the failed target
   from setup through webhook configuration, SMTP delivery, retry exhaustion,
   SIGTERM, EOF/reap and the final assertions. There is no bind-and-drop or
   other unowned interval in which a parallel test can claim the endpoint.
   Retain an executable ownership check appropriate to the chosen fixture
   that fails if that ownership is lost; source comments alone are insufficient.
2. A real socket connect probe, under an explicit deadline, demonstrates the
   target's failure during connect. The actual webhook delivery uses that
   same owned target and exhausts on connect failure. An accepted TCP
   connection followed by TLS, HTTP, read or response failure does not satisfy
   this criterion. Do not assume `ConnectionRefused` is the only portable
   connect error: an alternative must still prove the connect stage and fit
   the test's bounded waits. Merely removing the error assertion or ignoring
   the test does not satisfy this criterion.
3. The existing real-server assertions still execute: the stalled sink reads
   a delivery before SIGTERM; exhaustion reaches one; the process exits
   successfully; exactly one final `webhook dispatcher stopped` message has
   delivered=0, dropped=0, lagged=0, exhausted=1, aborted=1; exactly one
   exhaustion warning includes a transport cause beyond its display prefix;
   stdout contains none of hunter2, hunter3 or marker-secret. Synchronization
   uses observed events and deadlines, without sleep-as-synchronization or
   probabilistic retries.
4. Implementation changes are confined to what this macOS failure requires
   in the named test and any directly necessary test-fixture support.
   Reconcile contradictory portability wording in the owning spec/plan/tasks.
   Compare against the pre-intake candidate: no shipping production changes,
   no reopening findings 23.1–23.3 or 23.5 or the distinct webhook regression,
   and no dependency, allowlist, gate, bundle routing or accepted-decision edits.
5. The focused Linux test selects exactly one test and passes; the complete
   binary suite and every local house gate below exit successfully on the
   implemented candidate. Coverage must finish with zero missed lines,
   branches and functions and exit zero; a summary followed by timeout is
   insufficient. Record commands, candidate identity, platform and outcomes.
   Commit all scoped work unsigned before final verify/review; no push,
   merge, tag, publication, issue closure or decision acceptance.

## Frozen guarantees and proof ownership

No implementation boundary of 0001 (synchronous accept/store), 0002
(subscribe before checking) or 0003 (oldest-only pruning) changes. The
nearby contract is webhook shutdown observation under 0041/0048; SMTP
acceptance remains independent of webhook success. Preserve ingest extraction
under 0005 as well.

Local acceptance is limited to checkable Linux results and the scoped diff.
CI owns the actual macOS portability proof: its existing macOS binary test
must pass the repaired test without ignoring it. This is an external handoff
obligation, not a local acceptance criterion or a claimed macOS pass.
CI/operator also retain the charter's chart/history, Docker smoke, RustSec,
privileged port-80, release-only and live-hosting/settings proofs. Missing
external execution is recorded explicitly and does not block intake framing.
No GitHub, journal, sibling worktree or network inspection is needed.

## Verify commands

Focused and affected real-server suites:

```bash
cargo test --locked --test binary sigterm_with_a_delivery_in_flight_prints_the_final_counts -- --exact
cargo test --locked --test binary
```

Run all house gates with the candidate-local engine. Check its identity before
execution; its digest must equal both the value below and tooling.json:

```bash
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum --check --strict
sha256sum --check --strict bundles/self/scripts/shims.sha256
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh
bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh
bash scripts/seal-branch-test.sh
cargo deny check licenses
```

Under 0050 the verify seat checks the public shims and executes the corresponding
pinned `bundles/self/scripts/` copies against the candidate, offline, installing
nothing. The public spellings above identify the house inventory. Also run the
pinned seat's existing additional proofs:

```bash
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
```

Fail-closed native-tls absence proof (an arbitrary cargo error is not success):

```bash
python3 - <<'CHECK_NATIVE_TLS'
import subprocess
r = subprocess.run(
    ['cargo', 'tree', '--workspace', '--locked', '--offline', '--invert', 'native-tls'],
    text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
)
print(r.stdout, end='')
expected = "error: package ID specification 'native-tls' did not match any packages"
if r.returncode == 0 or expected not in r.stdout.splitlines():
    raise SystemExit('native-tls absence was not proven')
CHECK_NATIVE_TLS
```

Use this candidate's bundles and recipe routing unchanged. No benchmark is
required for this test-only repair. Intake changes no implementation and claims
no repaired-candidate or macOS pass; any intake gate receipts exercise the
existing Linux fixture and must not substitute for final verification.
