# Tasks — trustworthy gates, tests and delivery (#50–#53)

Ordered. Each task is paired with the test or gate that proves it (real
protocol against real servers, never mocks — 0004). Verify executes the
red proofs first, output captured, then reverts the plants; the permanent
proofs live in the committed suites and the gate's selftest. The exact
100/100/100 gate is the arbiter: any uncovered arm is dead or under-tested
and the fix is a test — `coverage(off)` is forbidden.

- [x] **1. Bind the feed's stop sender (the #50 coin flip)** — in the
  inline test `a_cleared_inbox_ends_the_feed_after_the_snapshot`
  (`src/feed.rs`), replace the `quiet_shutdown().1` temporary with a bound
  pair (`let (_shutdown_tx, shutdown_rx) = quiet_shutdown();`) so the
  dropped-sender watch cannot resolve and the `Closed` arm is the only
  ready branch. No production change; no `biased;`; no arm reorder (0021).
  *Test:* the inline test itself, run repeatedly (e.g. 50×
  `cargo test --lib feed` in a loop) — every run green; then
  `bash scripts/coverage-gate.sh` three times, three exact verdicts
  naming `src/feed.rs` covered. Proves AC50.1.
- [x] **2. Per-run evidence in the gate** — in
  `scripts/coverage-gate.sh`: after `mkdir -p target/coverage`, create
  `run_dir="$(mktemp -d target/coverage/run.XXXXXX)"` (NOT covered by the
  EXIT trap, which keeps deleting only the build dir); write lcov, summary
  and log there; point every later stage (leak check, declaration check,
  counting awk, summary jq, refusals) at `$run_dir/…`.
  *Test:* two concurrent gate runs each print/leave their own
  self-consistent evidence (verify captures both); a forced-red run names
  its own paths. Proves AC50.2.
- [x] **3. Harden the declaration check** — factor the source-tree checks
  into `check_no_coverage_off <dir>`, `check_harness_declarations <dir>`,
  `check_no_harness_dirs <dir>` (main path passes `src`); rewrite the
  declaration awk: every `mod <stem>;` declaration in every declaring
  parent must carry an attribute-stack guard whose trimmed line is exactly
  `#[cfg(test)]` (no first-match break; comment/doc/macro copies never
  count); refuse any `#[path = …]` on a harness-named declaration; refuse
  files under `src/**/{tests,examples,benches}/` (find, fail-closed).
  Replace the `git grep` with `grep -rn 'coverage(off)' src/
  --include='*.rs'` (≥2 = refuse) and delete the `GIT_CONFIG_COUNT=0`
  workaround.
  *Test:* the gate's new `selftest` subcommand (task 4) asserts every
  refusal fires on generated fixtures; the real gate stays green on the
  tree (`src/store/persist_tests.rs` is the live canary). Proves AC50.3,
  AC50.5.
- [x] **4. The gate's permanent selftest** — add a `selftest` branch to
  `scripts/coverage-gate.sh`: generate one hostile fixture per refusal arm
  in a mktemp tree (doc-string `#[doc = "#[cfg(test)]"]` guard; unguarded
  real decl + guarded copy in a comment/macro body; two parents, one
  unguarded; `#[path]` escape; harness-named file under `src/api/tests/`;
  untracked `coverage(off)`), run the factored checks against each, assert
  refusal + a clean copy passes, refuse fail-closed on any unexpected
  status.
  *Test:* `bash scripts/coverage-gate.sh selftest` is green on the tree
  and red when any check regresses (verify demonstrates one regression,
  captured, reverted); CI's coverage job and the operator's verify-seat
  diff run it. Proves AC50.4.
- [x] **5. Sinks that can see a fifth attempt (#51)** — in
  `tests/webhook.rs`, all three sinks: delete the `hits == expect` early
  fire; parameterize the window (budget tests ≈3 s, outliving the 2.1 s
  budget; others keep the constant); the count channel resolves only at
  window close with the final observed count; delete the 2800 ms sleeps
  and the dead `expect` parameter; assertions become
  `let seen = rx_count.await`.
  *Test:* the full-budget tests assert `seen == 4` with the window
  outliving the budget (a fifth attempt would red); a new observed-count
  leg closes the window before the first attempt and asserts `0`; verify
  demonstrates the deliberately-wrong-budget variant (`seen == 3`) red
  with captured output, then reverts. Proves AC51.1.
- [x] **6. Ports read back from the child; children killed on drop** — in
  `tests/binary.rs`: bind `:0`, pipe the child's stderr, parse the
  `Swarmail is up` line's `smtp =`/`http =` addrs, wrap the child in a
  kill-on-drop guard, delete the 23456–23473 block; `sigint_and_wait`
  keeps its 30 s liveness shape verbatim.
  *Test:* the whole `tests/binary.rs` suite green repeatedly with no
  hardcoded port; a deliberately failing assertion in a scratch run
  leaves no listening child (`ss`/`lsof` spot-check in verify, captured).
  Proves AC51.2.
- [x] **7. The three readiness gauges** — add `swarmail_awaits_active`
  (parked waits, drop-guarded inc/dec on every wait path REST and MCP
  share), `swarmail_http_requests_in_flight` (small axum middleware
  counter), `swarmail_deliveries_in_flight` (around the SMTP delivery
  insert, drop-guarded); append the three rows in the `/metrics`
  handler. Every new line is executed by the tests that poll it — the
  exact gate stays 100/100/100 with no exclusions.
  *Test:* the deadline-bounded `wait_for_gauge` helper (task 8) exercises
  all three over real HTTP; `/metrics` renders them (any existing
  metrics-reading test extended to assert the rows appear and move).
  Proves AC51.3 (instrument half).
- [x] **8. Replace the eleven sleeps with observed readiness** — convert
  lifecycle 315/346/382/408 + 451 + 556, api 579/614/641, mcp 1043 and
  binary 611 to `wait_for_gauge` polls (`awaits_active == 1` for parked
  waits, `http_requests_in_flight == N` for seeds,
  `deliveries_in_flight == 1` for the commit-burn case) before the stop
  or delivery under test. If any conversion surfaces a real in-flight-at-
  stop bug (a panic behind the closed listener), stop and name it as its
  own change under 0024 — never re-sleep.
  *Test:* the converted tests themselves — each now fails loudly if the
  condition never arrives (verify demonstrates one deadline red with the
  gauge name, captured, reverted); the stop-refusal assertions hold with
  the request provably arrived. Proves AC51.3.
- [x] **9. `http_json` fails loud on garbage** — in
  `tests/common/mod.rs`: parse the body; on failure panic naming status
  and body verbatim, except an empty body on 204/304 which stays absence.
  *Test:* a scratch demonstration (non-JSON body → red with the body in
  the message, captured, reverted) plus the suite green; the helper's own
  callers unchanged. Proves AC51.4.
- [x] **10. The persist spin-gate releases on every exit path** — in
  `src/store/persist_tests.rs`, guard `release` with a Drop guard set
  before the assertion.
  *Test:* a scratch demonstration of the failed-assertion path exiting
  promptly instead of hanging (verify, captured, reverted); the harness
  file stays 0020-sanctioned and the gate's harness checks stay green.
  Proves AC51.5.
- [x] **11. CI: supply-chain posture + honest comments (#52)** — top-level
  `permissions: contents: read` with per-job grants preserved; every
  third-party action pinned by SHA (`git ls-remote` each tag; trailing
  `# <tag>` comments); `rust:1-alpine` digest-pinned; delete the
  "decision 0063" citation; rewrite the coverage comment to describe the
  gate's actual nightly resolution.
  *Test:* `actionlint`-style local YAML check + the suite of jobs on the
  run's own push (verify captures the run's job list); a grep proving no
  floating `@v`/`@master` action refs and no "0063" remains. Proves
  AC52.2, AC52.4.
- [x] **12. CI: the pinned brokkr job + the 5k release job** — new
  `brokkr-compile` job reading `bundles/self/tooling.json` via jq
  (named refusal when absent/malformed), checking out
  feedback-loop-ai/brokkr at the recorded SHA, building, asserting the
  built `--version` equals the record, then
  `brokkr compile --bundle bundles/self`; docker + chart `needs:` it. New
  tag-scoped release job: `cargo test --release --test rate -- --ignored
  --nocapture`, `timeout-minutes: 15`, in no `needs:` list.
  *Test:* the jobs green on the run's push (with the operator's
  tooling.json applied) and the brokkr job's named refusal demonstrated
  with the record temporarily absent (captured, reverted); the 5k job's
  `if:`/`needs` shape asserted by grep. Proves AC52.1, AC51.6.
- [x] **13. The native-tls check fails closed in CI** — add to the `deny`
  job: run `cargo tree -i native-tls`; exit 1 = absent = pass, exit 0 =
  present = refuse, other = cargo failed = refuse (message distinguishes
  the three).
  *Test:* the deny job green on the tree; the three exit paths
  demonstrated (a scratch tree with native-tls present, a broken
  cargo invocation, the clean tree) — captured in verify. Proves AC52.3
  (CI half).
- [x] **14. Both tag forms publish; the guard probes both** — release
  push tags `:v<version>` and `:<version>`; `publish-guard.sh` probes
  both manifests (404/404 pass; 200 on either refuses named; unknown
  refuses); `publish-guard-test.sh` gains the both-tag cases.
  *Test:* `scripts/publish-guard-test.sh` green including the new cases;
  the workflow's tags block asserted by grep; the docs' `:v0.1.0` now
  names a tag CI publishes. Proves AC53.1.
- [x] **15. Chart: configurable ingress peers** —
  `networkPolicy.ingressPeers: []` values list rendered verbatim under
  `from:` (empty ⇒ exactly today's `podSelector: {}` shape);
  `scripts/chart-verify.sh` asserts the locked default and, in the full
  profile, that configured `namespaceSelector`/`ipBlock` peers render and
  are admitted.
  *Test:* `scripts/chart-verify.sh` green on both profiles; the red
  proof (drop a peer from the values or the assertion → chart-verify
  red) captured in verify and reverted. Proves AC53.2.
- [x] **16. Chart: mountable TLS** — `tls.enabled`/`tls.secretName`;
  the deployment mounts the Secret and sets `SWARMAIL_TLS_CERT`/
  `SWARMAIL_TLS_KEY` to the mounted paths when enabled; unset renders no
  volume, no mount, no TLS env. chart-verify asserts the wiring in the
  full profile and the plaintext absence in the default profile (cert
  lifetime never asserted — 0020 rule 5).
  *Test:* chart-verify both profiles; a scratch render with a fake
  secret name shows the exact mount + env paths. Proves AC53.3.
- [x] **17. Chart: restore-sized startupProbe + the memory relation** —
  startupProbe on `/healthz`/http with a budget documented against a full
  restore of the default PVC (values + comment); the memory relation
  documented beside `resources` (limit ≥ restored state;
  `SWARMAIL_MAX_PER_INBOX` bounds it); no conditional override;
  restore-before-bind untouched. Decision 0026 carries the contract.
  *Test:* chart-verify asserts the startupProbe present with its budget
  and the relation comment present in values; the deployment contract's
  other probes unchanged. Proves AC53.4.
- [x] **18. Hygiene: .dockerignore, NOTES.txt, Chart license** — exclude
  `.forge/` and `*.db` from the build context; NOTES.txt names the
  generated claim only when generated; Chart.yaml's
  `artifacthub.io/license` becomes `MIT OR Apache-2.0` (0006).
  *Test:* chart-verify render asserts the NOTES conditional both ways;
  a `docker build` context check (or CI build) proves `.forge/` and
  `*.db` absent; grep on Chart.yaml. Proves AC53.5.
- [x] **19. Operator edits recorded** — write `.forge/run6-operator-edits.md`:
  the verify-seat.sh unified diff (add `publish-guard-test.sh` + the gate
  selftest; document chart-verify as CI-only; fail-closed native-tls)
  and the `bundles/self/tooling.json` record, each with a one-paragraph
  rationale. Run-local evidence — never committed.
  *Test:* the diffs apply cleanly against the current files (`patch
  --dry-run`); the sidecar record compiles
  (`brokkr compile` against a copy — never the live bundle). Proves
  AC52.3 (seat half) and the R11 record.
- [x] **20. Final gates** — `cargo fmt --all -- --check`;
  `cargo clippy --workspace --all-targets --locked -- -D warnings`;
  `cargo test --workspace --locked`; `bash scripts/coverage-gate.sh`
  **three times** (exact, plus one concurrent pair with distinct
  evidence); `cargo deny check licenses`; `brokkr compile --bundle
  bundles/self`; `bash scripts/coverage-gate.sh selftest`.
  *Test:* the gates themselves — the run's closing evidence. Proves the
  standing-gates clause of the spec.
