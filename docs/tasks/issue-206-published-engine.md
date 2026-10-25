# Issue #206 intake — admit the published Brokkr engine

Run: `epic-192-issue-206-pin-one-repro-dcc89977`  
Phase: intake; framing resolved, implementation and verification outstanding.  
Candidate inspected: `c3d124d7217e9303d75a6a524f2babcc35078937` on the supplied rollup-a base.

## Goal

Make CI and the hermetic verify seat use the same published Brokkr executable, admitted by its recorded SHA-256, and reject the same policy-sweep warning.

## Authority and evidence

The supplied `.forge/epic192/issue206/README.md` and `operator-rulings.md`, including release records and reference patches, pass that directory's `SHA256SUMS` (checked from that directory). The operator's 2026-10-24 #206 ruling chooses publication rather than reproducible builds. The published artifact is already available according to the checksummed release record; intake made no network request.

- Source base: `731189f9097308c01ae4f2b4c9caea5f41224844`, version `0.12.0`.
- Artifact: `https://github.com/feedback-loop-ai/brokkr/releases/download/swarmail-engine-731189f-p3/brokkr-0.12.0-731189f-p3-x86_64-unknown-linux-gnu`.
- Binary SHA-256: `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`; candidate-local `.forge/bin/brokkr` was independently hashed and matches.
- Ordered patches already recorded in `bundles/self/tooling.json`: `0001-preserve-review-residual-facts.patch` (`5d4da2217135c5a2ca14354ae9907623cf67ff0076b078277cb77a09ad3020b5`), `0002-preserve-review-scope-and-fixes.patch` (`361272eff344a5d5e05b832281407a99c3f1a77fc1fbc5216e19fb4ce7487173`), `0003-bind-verified-candidate.patch` (`4098228e7649d1bda8961bb491aa29d982b462c420470e265f8b9acf0a148d96`), all relative to `bundles/self/tooling-patches/`.

Read the decision index and relevant 0019 §1, 0045, 0050, 0056, 0057 and proposed 0072. This is compatible enforcement of accepted tooling/proof identity, not a semantic exception or decision acceptance. No frozen guarantee 0001–0003 is directly changed; synchronous insert, subscribe-before-check and oldest-only pruning remain intact. Runtime, extraction and dependency policy are outside scope.

Spot-checks confirm CI currently compiles four recipes with its source-built executable and does not admit its digest or refuse sweep warnings. Verify already checks the launched digest but describes a build of the source SHA. `tests/delivery_constitution.rs` already has an executable missing/mismatched-engine fixture, so strengthen it rather than duplicate it. Recent relevant history includes #171 (`3b8ca6b`), #172 (`439ad8e`, `c3d124d`) and the reconciled tooling pin (`1fdae52`). Their seat/preflight/non-login and proof changes must survive. Reference patches in the evidence packet are not candidate patches to apply wholesale.

## Checkable acceptance criteria

1. `bundles/self/tooling.json` retains the exact base, version and ordered checksummed patches above, records the exact published artifact URL and binary digest above, and describes a published build from the base plus those patches. Its provenance no longer points to an unavailable receipt as the only acquisition route or presents the patched binary as an unmodified base build. A record assertion fails when URL, digest, base or patch order differs from the authorized record.
2. `.github/workflows/ci.yml`'s `brokkr-compile` validates the URL and `binary_sha256` before exporting values; absent/malformed values refuse by name. Values enter `run:` bodies through `env:` only (0019 §1). The job downloads that URL, checks SHA-256 and emits a named digest-mismatch refusal before any execution of downloaded bytes. All four existing recipe compiles (`self`, `codex`, `codex-careful`, `codex-fast`) use the downloaded, checked executable. Any retained source checkout/patch/aggregation tests remain auxiliary and cannot substitute their build for the artifact. Local checks exercise the mismatch branch and assert that compilation was not reached; the real download is CI-owned.
3. CI captures compile diagnostics without losing a failing compile status and refuses a successful compile containing `lets the run go on without reading`, matching verify's existing policy-sweep check. A warning fixture exits nonzero; a warning-free successful control proceeds; a failing compile remains a failure.
4. `bundles/self/scripts/verify-seat.sh` identifies its check as comparison of the launched executable's digest with the recorded `binary_sha256`, including missing-digest and mismatch refusals. It no longer claims digest equality proves an unpatched source-SHA build. It still refuses identity failure before any gate and continues to use its pinned resources. The hermetic seat checks the pre-provisioned candidate-local artifact; acquisition is the operator's setup/CI responsibility, not a new network dependency inside verify.
5. Extend `verify_seat_refuses_missing_or_mismatched_engine_identity_before_gates` to assert the revised named digest refusal in actual result JSON and that no gate ran. Keep the missing-digest case and digest-matched successful control. Disabling the comparison or admitting the mismatched executable makes the regression fail.
6. Refresh/check `bundles/self/scripts/shims.sha256` alongside canonical script changes and update the independently reviewed digest table and refusal assertions in `tests/delivery_constitution.rs` in the same implementation commit. The manifest still names exactly the seven public shims; unchanged shim bytes retain their digests. Both the manifest checks and canonical script/tooling digest pins pass. Preserve inherited recipe routing, protected review, security hold, public-shim refusal and #171/#172 proof ownership.

## Verification commands and ownership

Implementation/verify must run all house gates on the committed candidate, offline, installing nothing. Use only the candidate-local `.forge/bin/brokkr` with the exact digest above; no host-PATH or sibling engine. These commands are completion requirements, not claims of an intake pass:

```bash
export CARGO_NET_OFFLINE=true
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum --check --strict
cargo test --locked --test delivery_constitution
cargo test --locked --test delivery_recipes
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh
bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh
bash scripts/seal-branch-test.sh
cargo deny check licenses
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
sha256sum --check --strict bundles/self/scripts/shims.sha256
```

Additionally compile `bundles/codex`, `bundles/codex-careful` and `bundles/codex-fast` using `.forge/bin/brokkr`; capture combined output for every recipe and reject the exact sweep-warning substring above even on exit 0. Execute the fail-closed dependency proof:

```bash
out=$(cargo tree --locked -i native-tls 2>&1); code=$?
if [ "$code" -eq 0 ]; then
  printf 'REFUSAL: native-tls present\n%s\n' "$out" >&2; exit 1
elif printf '%s\n' "$out" | grep -q 'did not match any packages'; then
  printf 'native-tls absent\n'
else
  printf 'REFUSAL: absence unproved (exit %s)\n%s\n' "$code" "$out" >&2; exit 1
fi
```

The hermetic verify driver must execute dispatched pinned canonical scripts, after checking public shims, rather than select its algorithm from candidate wrappers. Coverage must prove exact zero missed lines, branches and functions, and its selftest must pass. No benchmark or application-protocol-specific filter is added for this tooling-only change; the full workspace suite retains the real-server coverage.

CI owns the actual published-asset download and execution on the PR, plus the charter's existing CI-only chart/history/selftest, Docker, network RustSec, tag-only rate and privileged port-80 proofs. The operator owns publication (already recorded), setup acquisition, sealing and live settings. No macOS, namespace, live GitHub-settings or network proof is imposed on this offline seat. Missing CI execution is reported as CI-owned pending evidence, not a local reproduction failure.

## Scope boundaries and handoff

Do not reproduce the historical source composite from a Git-less archive, require cold byte-identical builds, read GitHub/journals/sibling worktrees, fabricate journal evidence, change routing, accept a decision, publish, push, merge, tag or close an issue. No new dependency is needed. Use current candidate routes as recorded in `bundles/`; this intake does not select the next phase. Design should specify the exact artifact-record schema, named refusals and execution fixtures within the criteria above.

Intake records this framing only. Implementation, all completion gates and CI receipts remain outstanding; an intake `resolved` is not feature completion. Commit the framing unsigned under 0056, and commit implementation before verification.
