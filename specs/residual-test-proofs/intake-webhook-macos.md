# Intake: give the webhook refused-target test the macOS-portable fixture

Status: resolved framing; implementation and final verification remain outstanding.
Run: `epic-192-issue-191-give-the-webh-4ea45493`.
Date: 2026-10-25. Intake baseline: `ac160ff4a74a975226d58dd447f5f107327694f6`.

## Goal

Replace the unconnected reservation in `tests/webhook.rs::refused_connection_never_blocks_the_smtp_path` with the connected-client fixture already used by the SIGTERM test, preserving the exact refusal and SMTP/storage/retry assertions.

## Authority and bounded inspection

Read `.forge/epic192/issue191/README.md` and `operator-rulings.md` first.
All five entries of that directory's `SHA256SUMS` passed when checked from
that directory, including those two files and `macos-ci-238.md`. An initial
check from the repository root used the wrong relative directory; the
successful directory-local check is the evidence verification.
The supplied PR #238 evidence establishes that the SIGTERM fixture passes
macOS while this webhook test reports TimedOut instead of ConnectionRefused.
The operator's #191 ruling assigns macOS execution to CI. Apply that ruling;
no further acceptance or external inspection is needed.

Read the house charter, decision index, frozen 0001–0003, real-protocol
isolation 0004, regression placement 0020, webhook boundary 0041, pinned
executed proofs 0050 and unsigned policy 0056. Spot-checked both named
functions and local fixture history (db0b517, 35afa14, 94b8ae1, 7266cf9).
The webhook test still holds an unconnected TcpSocket; the SIGTERM test
connects a reuse-disabled client to a temporary listener, accepts its peer,
drops the listener and holds both connected ends through its final checks.
Its comment explicitly limits macOS reuse claims. No implementation is
changed by intake. Existing intake and design receipts remain historical.

## Checkable acceptance criteria

1. In the named webhook test, explicitly bind a reuse-disabled IPv4 client,
   connect it to a temporary loopback listener, accept the peer and drop the
   listener, following the existing SIGTERM fixture. Use the connected
   client's local address as the webhook target. Retain the connected client
   AND accepted peer until after the final storage assertion, then drop both.
   Bound setup connect, accept and refusal probe with HELPER_DEADLINE.
   Source inspection fails if the target remains an unconnected reservation,
   is released early or differs from the address actually probed.
2. Retain the real TcpStream connect probe and exact
   `std::io::ErrorKind::ConnectionRefused` equality. A timeout, accepted
   connection, TLS/read failure, broader error match, new ignore, sleep for
   synchronization or probabilistic retry cannot pass. Add no SO_REUSEADDR
   contender assertion. State the existing SIGTERM limitation: macOS
   reuse-enabled listeners, including parallel Tokio listeners, can still
   claim the client endpoint; the probe observes setup only and does not
   prove refusal on every subsequent webhook attempt or close that race.
3. Preserve the test's real PUT success, pre-mail exhaustion baseline,
   successful SMTP delivery, count exactly one before observing exhaustion,
   one new exhaustion over that baseline and count still exactly one after
   retries. The focused test must select exactly one passing test, and the
   complete webhook and binary suites must pass locally with their existing
   ignore policy. Preserve the SIGTERM test byte-for-byte.
4. Inspect the tests changed by #191 (tests/webhook.rs, tests/binary.rs,
   src/api.rs test module and tests/common/mod.rs, using the local change
   history) and record the inventory: no refused target introduced or
   modified by #191 may remain an unconnected reservation. Inspect functions,
   not just a text search result. Do not expand this into repairing unrelated
   older refused-target fixtures. Intake's bounded search found the webhook
   reservation and the already-connected SIGTERM fixture; final verification
   must confirm the repaired inventory.
5. Relative to the intake baseline, implementation changes are confined to
   the named webhook function's fixture, lifetime and directly related
   comments/diagnostics. The owning design may reconcile only directly
   contradictory requirements. Intake/verification records may be added.
   No production, unrelated tests/helpers, dependency, lockfile, allowlist,
   accepted-decision, gate or bundle/recipe-routing edits. Enforce this with
   the baseline diff; commit all scoped work unsigned before final verify.
6. Every local house gate below must exit zero on the committed repair.
   Coverage must complete with zero missed lines, branches and functions AND
   a passing selftest; a summary followed by timeout is not success. Retain
   platform, command, candidate commit/tree, exit and output receipts.
   Intake resolution alone certifies framing, not feature delivery.

## Guarantees and proof ownership

Adjacent to frozen 0001: the real SMTP success still means stored and
queryable even when webhook delivery fails. This is fixture-only work
within accepted semantics; it does not change 0001, 0002's subscribe-before-
check, 0003's oldest-only pruning or 0005's extraction at ingest. No semantic
decision document or decision acceptance is required.

CI owns execution of the repaired webhook test on macOS. This Linux seat
neither claims that result nor makes it a locally provable criterion.
CI/operator retain chart/history proofs, Docker smoke, network RustSec,
privileged port-80 exchange, release-only proofs, live GitHub settings and
sealing/publication. No GitHub, CI, journal, sibling worktree, namespace or
network inspection is authorized. No workspace input is missing for framing.

## Verify commands

Run offline with cached tools; install nothing. No benchmark is warranted.
Keep candidate bundle routing unchanged. Check the engine and public shims;
under 0050 run canonical pinned script copies against the candidate (the
public scripts/ dispatchers identify the same house gates).

```bash
cargo test --locked --test webhook refused_connection_never_blocks_the_smtp_path -- --exact
cargo test --locked --test webhook
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

Reject compile policy-sweep warnings even on exit zero. Prove native-tls
absence fail-closed, accepting only Cargo's specific missing-package error:

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

## Intake execution

Evidence checksums and candidate-local engine digest match. The initial
worktree was clean. Gate results for this documentation-only intake will be
recorded below; they exercise the existing Linux implementation and cannot
certify the future repair or macOS. Final implementation must rerun them.

### Intake baseline gate receipt — 2026-10-25

All 17 recorded commands exited zero on Linux x86_64 at committed framing
`039ef063a2ec12304afe3726ab5156b652b02457`, tree
`e462045fdea0244fa9e38d01daf6d5beaad4fabe`: engine/shim integrity,
focused webhook (exactly one pass), full webhook (27 passes), full binary
(40 passes, two existing ignores), fmt, locked clippy and workspace tests,
pinned local engine compile (76 valuations, zero unruled, no sweep warning),
exact coverage, coverage selftest, publish/seal proofs, licenses, design
selftest, delivery tamper proof and strict native-tls absence. No new ignores.
Coverage completed with 14030/14030 lines, 790/790 branches and 1531/1531
functions; gate exit zero and evidence `target/coverage/run.jYgUMJ`.

The workspace RPC timed out at 300 seconds; the process completed in 405.58
seconds. A subsequent workspace read confirmed every saved exit and the
final `all_passed: true` receipt and `done` marker. The RPC timeout is not
counted as success. Commands, candidate identity, platform, timestamps and
per-command output are retained under
`.forge/epic192/issue191/intake-webhook-macos-gates/receipt.json` and adjacent
logs. The initial detached runner produced no receipt; the foreground run
owns these results. The worktree was clean after the gates. This addition
records results only, changing no tested code. These Linux baseline receipts
do not certify the future fixture change or the CI-owned macOS proof.
