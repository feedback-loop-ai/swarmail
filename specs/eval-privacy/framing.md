# Intake framing — port issue #196 privacy onto issue #194

Run: `epic-192-issue-196-keep-the-call-1613a8c3` · 2026-10-24 · intake only.

## Goal

Port every verified #196 environment, diagnostic and artifact privacy property onto #194's matrix-cancellation runner while preserving both changes' regression suites and all cancellation, cleanup-ordering and accounting behavior.

## Evidence and authority

- Read `.forge/epic192/issue196/README.md` first; verified all ten entries with `(cd .forge/epic192/issue196 && sha256sum -c SHA256SUMS)`, all OK.
- Use the eight ordered `patches/0001…0008` as behavior/design evidence, with `prior-own-diff.patch` as supplementary evidence; ignore the duplicate `prior-patches/`. Historical pass receipts establish the source of the port, not verification of this candidate.
- Base observed at `509a0c7` (renumber matrix cancellation to 0073), with a clean worktree before intake. Local history includes `bce47e9` (prove cancellation teardown ordering). Do not replace this base with the old privacy runner.
- Read the house rules and decision index, accepted 0055 (standalone eval crate and runtime contract), accepted 0058 (review routing), proposed 0073 (cancellation), and the packet's proposed privacy decision. Spot-checked runner spawning/draining, Admin::call, write_artifacts, run_trial, matrix run_child and the pinned verify inventory.
- Verified `.forge/bin/brokkr` SHA-256 is `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`, matching `bundles/self/tooling.json`.

No journal, sibling worktree, GitHub or CI access is needed or authorized. No missing local input prevents framing. Actual macOS execution evidence required by the inherited privacy specification must be supplied during delivery; a Linux pass cannot stand in for it.

## Frozen guarantees and decision boundary

No direct change to 0001 (accept == stored), 0002 (subscribe before check), or 0003 (oldest-only pruning) is authorized. The work sits in the eval lifecycle beside accepted 0055 §§6–10; root server semantics, ingest extraction (0005), dependencies, exact coverage policy and delivery policy stay intact. Test seams remain test-only under 0020. The privacy proposal compatibly strengthens 0055 and does not authorize weakening an accepted ruling.

Design must carry the packet's `0062-eval-privacy.md` into `docs/decisions/0075-eval-privacy.md`, status **proposed**, with a header explicitly saying **renumbered from 0062**, corrected live links and a proposed index entry. Do not reuse 0062–0074, alter accepted decisions, or accept 0073/0075. Adding this record belongs to downstream design/implementation, not this intake-only artifact.

## Scope

Primary owners: `evals/src/{runner,main,mcp,oracle}.rs`, `evals/tests/` and owning inline tests, `evals/README.md`, the owning eval specification/plan, privacy design artifacts and proposed decision/index. Preserve #194's structure and transplant #196 behavior. Patch 0008 documents an already-upstream routing assertion repair; do not reintroduce its stale failure or expand into recipe changes.

No production root-server changes, dependency widening, raw-output option, credential forwarding API, provider change, new confinement claim, publication or release work. Commit unsigned only if policy reaches ship; never push, merge, tag, publish or close issues.

## Acceptance criteria

These criteria supplement and retain all applicable packet privacy AC1–AC23 and #194's cancellation criteria. Design must trace each inherited criterion/test to its ported counterpart; do not silently drop a test because its old structure conflicts.

1. **Closed environments for every role.** Real command observations compare complete environments for Swarmail, fixture, agents (oracle/null included), matrix workers and Cargo preparation. Every command starts cleared, with only the documented fixed/private map and role-specific generated inputs. Arbitrary names, provider credentials, stale EVAL/SWARMAIL values, proxies, loaders, shell startup values and non-UTF-8 synthetic canaries do not cross. Caller HOME/PWD are absent; generated private HOME/TMPDIR/four XDG directories are distinct per child under fresh 0700 `/tmp` ownership. Agents receive all eight correct EVAL inputs and identical prompt-file/EVAL_GOAL bytes. Set generated agent PWD to its actual workdir and assert equality in the env-echo test; this adds no inherited value. Test controls are never runtime map exceptions.
2. **Deliberate lookup and preparation.** Preserve original-cwd/PATH executable lookup before clearing/changing cwd, absolute/relative/bare programs, unset/relative/empty PATH components, executable checks and literal later argv, including native non-UTF-8 paths/arguments through the real matrix CLI. Fallback builds remain once per batch, offline/locked, with custom-target executable discovery and only the packet's named preparation inputs. Both explicit Cargo/Rustup homes work with HOME removed; missing required defaults and empty selectors refuse safely. Preparation locators never flow into runtime roles or metadata. Synthetic wrapper/config authentication succeeds without inheriting credentials; no secrets enter argv or tracked evidence.
3. **Direct HTTP and inactive production fault controls.** Every reqwest client in evals explicitly uses `.no_proxy()`. Real oracle/proxy tests pass with dead HTTP_PROXY/HTTPS_PROXY/ALL_PROXY settings; loopback admin Authorization never goes to a proxy. Production trial/matrix binaries ignore former runner/test controls, including EVAL_RUNNER_FAULT=panic/server_death. Retain #194's test-only cleanup/signal seams and their executed proofs without exposing them in production or forwarding them to runtime children.
4. **Safe publication at every sink.** Drain arbitrary child stdout/stderr concurrently and retain/publish none of that text. Independently emitted synthetic secrets, bearer tokens, paths, arguments, panic payloads and build/worker errors are absent from CLI stdout/stderr, diagnostics, metadata and every retained/temporary artifact. Emit only finite harness-authored categories and numeric facts; diagnostics.log stays bounded and metadata.swarmail_bin is null. CLI parse failures and panic hooks follow the same rule. Admin transport/read/parse/non-200 failures suppress response bodies, URLs and error text, proven with injected response canaries. Preserve explicit task/agent labels, schema values and honest numeric exit/cleanup facts rather than scrubbing coincidental matches.
5. **Bounded drains without disrupting cancellation.** Retain only the first typed loopback readiness announcement, with an 8192-byte line bound and bounded nonblocking producer storage; pre/post-readiness floods keep draining. Cargo records cap at 1 MiB. Preserve #194's stricter 64 KiB matrix row/output retention and its validation, instead of importing the older 1 MiB matrix cap. Oversized/malformed/extra/mismatched rows, wrong labels, row/worker exit mismatches and CR controls refuse without echoing bytes. Prove bounded retention under counted producer/completion evidence, output pressure and independent short controls; cancellation and cleanup remain runnable.
6. **Private artifacts before payload.** A fresh trial leaf is exactly 0700; all four ordinary retained files (state.json, result.json, diagnostics.log, metadata.json) and owned temporary files are exactly 0600, under permissive and owner-bit-removing umasks. Existing parent modes remain unchanged. Tests inspect actual creation before payload and deterministically exercise write/publication failures. Post-write chmod alone fails this criterion. Require a real failed trial with snapshot, Fail/GaveUp, agent status 0 and CLI exit 1 so absent artifacts cannot pass.
7. **Exclusive atomic publication.** Refuse an existing trial leaf directory/file/live or dangling symlink; exclusive temporary creation and no-clobber final publication preserve sentinel contents/modes across file/directory/symlink/hard-link conflicts. Final names appear only after complete private payloads; cleanup removes only owned names. Artifact failures retain established result/exit semantics and the safe artifact_write_failed category. Prove actual selected primitive semantics on Linux and macOS separately; unavailable macOS execution remains explicitly outstanding.
8. **Cancellation ownership and ordering unchanged.** Retain #194's pre-spawn signal subscription, latched SIGTERM/SIGINT admission stop, once-only forwarding, owned fallback Cargo cancellation/reaping, body-admission closure/join, retained wait/group ownership, concurrent five-second TERM grace then KILL and five-second observation/reap bound, and final cleanup fence on normal and interrupted completion. A live descendant, EOF, exited leader or successful kill request alone cannot prove cleanup. Unknown observations/expiry remain honest harness failures. Repeated signals do not reset/shorten cleanup. Keep platform-specific identity/zombie observation behavior and early-handler ordering.
9. **Matrix accounting and failure precedence unchanged.** Preserve schema 1, task → agent → repetition full cardinality, completed prefix, one post-cleanup active error row, fresh zero-time refused suffix IDs without launches, cancellation-winning commitment races, late-signal exit 2 without rewriting committed rows, real observed statuses/lifetimes and output-failure admission stop. Keep exactly-one matching-row validation, independent child wait and #194's output-progress behavior. Report attempts and success formulas, snapshot-before-shutdown/scoring-after-cleanup, and infrastructure/deadline/business precedence remain unchanged; every result remains unconfined.
10. **Both suites survive the port and discriminate.** Inventory every #194 test and every packet #196 test, retain or adapt each with named mapping, and run real subprocess/SMTP/HTTP/MCP proofs. Include both signals, acknowledged cleanup hold/release and cleanup-await negative control, TERM-resistant descendants, startup/completion races, repeated signals, bootstrap/output failures, privacy env/output/artifact mutation controls, server death, panic, deadlines, oracle-all-pass, null declared idle failures, and overlapping isolation. Deterministic seams/acknowledgements replace sleep synchronization. Each negative control must make its relevant assertion fail; revert mutations before verification.
11. **Documentation and proposal agree with behavior.** Update inheritance/transcript promises and live privacy links, document exact role/preparation maps, generated PWD, safe diagnostic loss, explicit wrapper authentication and unconfined limitations. Add/index proposed 0075 with renumbering header. Keep proposed 0073 and accepted 0058 intact. Documentation/status/link checks must fail if the old decision path, inheritance promise or accepted privacy status returns.
12. **Fresh candidate-bound delivery proof.** All commands below pass on the final candidate through the pinned proof chain, with exact coverage missed counts zero for lines/branches/functions and its selftest passing. Evals has its own independent fmt/clippy/tests evidence. Obtain five separate accepted-0058 careful results from the existing bundles: Astra correctness, official DeepSeek Flash security, Sonnet 5.5 tests, Astra guarantees and independent Fable 5.1. No substituted/default/combined reviewer or DeepSeek Pro; protected review/security hold remains binding. Prior runs are not fresh reviews or current gate receipts. Missing/failing proof remains outstanding, never completion.

There is no demonstrated behavioral contradiction at intake: generated PWD preserves caller exclusion, and the stricter #194 matrix bound satisfies bounded-output privacy. If design finds a true contradiction between required behaviors or an accepted ruling, stop and record the exact conflicting requirements; do not choose a weaker contract.

## Verify commands and proof ownership

Intake records these obligations and does not run build/test gates or claim feature completion. Verify executes offline, installs nothing, and uses `.forge/bin/brokkr` with the digest above. Check its identity and the bundle's tooling/shim identities before running. The public script commands below name the house inventory; the hermetic verify seat validates fixed shims and executes the corresponding **pinned bundle copies** against the candidate, as required by 0050.

```bash
sha256sum .forge/bin/brokkr
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

Also retain the pinned verify seat's additional deterministic proofs:

```bash
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
```

For native-tls absence, use the pinned verify-seat.sh fail-closed probe: `cargo tree -i native-tls` must fail specifically with `did not match any packages`; success or another error refuses. Do not treat an arbitrary nonzero exit as absence.

Independent eval checks (standalone workspace, not covered by root cargo test):

```bash
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --workspace --all-targets --locked -- -D warnings
cargo test --manifest-path evals/Cargo.toml --workspace --locked
```

The full eval command must include all inline/unit and integration targets, particularly runner, report, proof, fixture, boundary and scoring. Focused proof commands for iteration and reviewer reproduction:

```bash
cargo test --manifest-path evals/Cargo.toml --locked --test runner
cargo test --manifest-path evals/Cargo.toml --locked --test report
cargo test --manifest-path evals/Cargo.toml --locked --test proof
cargo test --manifest-path evals/Cargo.toml --locked --test report matrix_sigterm_and_sigint_wait_for_the_active_trials_owned_cleanup
cargo test --manifest-path evals/Cargo.toml --locked --test report matrix_signal_cancels_and_reaps_the_fallback_cargo_group
cargo test --manifest-path evals/Cargo.toml --locked --test runner normal_trial_binary_ignores_the_former_runner_fault_control
```

Design must name the adapted privacy filters/mutation commands from its retained-test mapping. Full eval tests and actual macOS toolchain/artifact proofs remain mandatory even after focused passes. No new benchmark/performance claim requires cargo bench.

House-rule CI-only chart/history/selftest, Docker smoke, network advisory audit, tag-only release-rate proof and privileged port-80 evidence remain CI-owned; report them separately from offline verification, without accessing CI or claiming they ran here. Do not republish historical pass claims as current proof.

## Intake outcome

Resolved: local evidence is sufficient to frame compatible privacy strengthening, with checkable preservation criteria and the complete downstream proof obligations. No implementation, decision acceptance, review invocation, gate-pass claim or shipping action occurs in intake.
