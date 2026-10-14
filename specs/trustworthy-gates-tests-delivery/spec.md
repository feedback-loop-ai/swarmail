# Trustworthy gates, tests and delivery — issue #58 children #50–#53

Feature: finalize the "Make the gates, tests and delivery trustworthy" section
of the codebase review (issue #58) — four children, delivered together
because they certify the same signal from four sides:

- **#50** — the exact coverage gate must be deterministic (no coin-flip cold
  line) and must close its declaration loopholes, because a gate that
  sometimes lies trains *rerun-until-green*, and rerun-until-green launders
  every future red, including the ones that mean it.
- **#51** — vacuous and timing-dependent tests must be able to fail: a count
  channel that fires before the window closes cannot observe the fifth
  attempt it claims to exclude; a sleep that fakes "the server has my
  request" makes the stop-refusal checks behind it vacuous.
- **#52** — CI, the verify seat and the documented gates must agree, under a
  brokkr pinned by commit (version strings demonstrably do not identify
  behavior: two 0.12.0 binaries, one refusing the bundle).
- **#53** — the published image tag and the Helm chart must match their
  docs: an image tag that does not exist is a deploy-time failure discovered
  by whoever needed the sink *now*.

Why together: each is a certifying machine — the gate, the suite, CI+seat,
the deployment — and a certifying machine obeys 0001's own law generalized:
*a gate that sometimes lies about coverage is worse than no gate; a test that
cannot fail is a 250 for evidence that does not exist.* None of the four
touches a frozen store guarantee; all four touch the machinery that proves
them.

## Positions and rulings

Both seats (`​.forge/design/positions/simplicity.md`,
`.forge/design/positions/robustness.md`) converge on the load-bearing shapes:
the feed flake is one line of test code; the gate's evidence must be per-run;
the declaration check must parse attributes; every check must distinguish
*checked-and-clean* from *could-not-check*; tests gain can-fire legs; brokkr
is pinned by SHA; the chart gets configurable peers, mountable TLS and a
restore-sized startupProbe; both image tags publish. They conflict on five
points. Each ruling names the winner and what the losing side costs.

**R1 — The feed cold arm is a one-line test fix; no production reordering**
*(both converge).* `a_cleared_inbox_ends_the_feed_after_the_snapshot` passes
`quiet_shutdown().1` — a temporary — so the watch sender dies at the end of
the statement and *both* `tokio::select!` arms are ready on the poll after
the clear (the 0021 drain closes the channel *and* the dropped-sender watch
resolves); `tokio::select!` picks a ready branch at random, so
`src/feed.rs` `Err(RecvError::Closed) => None` is hit on a coin flip. The fix
binds the sender in scope (`let (_shutdown_tx, shutdown_rx) = ...`) so
`changed()` provably pends and the `Closed` arm is the *only* ready branch —
branch-stable, inline, exactly 0020 rule 1's shape. Refused: `biased;` or arm
reordering in `feed_events` (a semantic change to the close-vs-stop posture,
0021 territory) and any gate retry of a red verdict (a retrying gate
launders its own cold-line misses into green). Cost of refusing the reorder:
the production code is already correct, so nothing is lost.

**R2 — The verdict is computed from the report the run itself produced**
*(both converge; robustness's diagnosis is the sharper one).* Today the gate
builds into a run-unique target dir but copies its lcov report to the
*shared* `target/coverage/lcov.info`, and every later stage — the harness
leak-check, the declaration check, the counting awk, the summary `jq` — reads
that shared path. Two concurrent runs can each write it: a red verdict can
quote the other run's missing lines. The design: a per-run evidence
directory `target/coverage/run.XXXXXX` (mktemp under `target/coverage`,
*not* deleted by the EXIT trap — the trap keeps deleting only the build
dir), written before the verdict; every refusal names its own run's paths.
No retention scheme: the dirs are gitignored, `cargo clean` reaps them, CI's
`upload-artifact` (path unchanged: `target/coverage`) is the durable copy.
Cost accepted: run dirs accumulate under `target/coverage/`.

**R3 — The declaration check parses declarations, and every one of them**
*(direction converges; the strictness is robustness's hostile-input list,
sharpened by the synthesis into an every-declaration rule).* For each
harness-named file under `src/` (`tests.rs`, `*_tests.rs`, `*-tests.rs`):

- a guard is a line that *is* `#[cfg(test)]` (trimmed) in the attribute
  stack directly above a declaration — a `#` line merely *containing* the
  text (`#[doc = "#[cfg(test)]"]`, a commented copy) is not a guard;
- **every** `mod <stem>;` declaration in **every** declaring parent must be
  guarded — no first-match. This is what kills the loophole the task
  reproduces: an unguarded real declaration cannot be authenticated by a
  guarded copy sitting in a comment or a macro body, because the real
  declaration is itself unguarded; and lib.rs can no longer vouch for
  main.rs declaring the same file unguarded (today's check `break`s on the
  first declaring parent);
- a `#[path = "…"]` attribute on a harness-named declaration refuses
  outright — the reserved-name rule is about file names, repointing is an
  escape;
- production files under `src/**/tests/`, `src/**/examples/`,
  `src/**/benches/` refuse (find-based, fail-closed on find error) —
  cargo-llvm-cov 0.9.1 drops those files from the report entirely, so the
  `SF:`-based leak check cannot see them;
- the `coverage(off)` check becomes plain recursive `grep` over `src/`
  (untracked files included by construction), failing closed on grep error;
  the `GIT_CONFIG_COUNT=0` workaround dies with `git grep`.

Cost accepted: a stricter check refuses shapes a human thinks are fine —
loudly, naming the file and the rule.

**R4 — The gate's strictness arms get a permanent proof; the chief rules
against the simplicity seat's cut here.** Simplicity would prove the refusal
arms once in verify (planted fixtures, captured output, reverted) and ship no
fixture tree and no source-root override — both refusals stand (a shipped
fixture tree is a maintenance surface; an env-overridable source root is a
vacuous pass waiting to happen). But the refusal arms are *cold paths*: they
run only when hostility is present, and on a clean tree a regression toward
leniency is invisible to every gate forever. 0020's own logic — cold-path
presence must be provable — demands more than a captured journal entry that
dies with the run. The house precedent is `scripts/publish-guard-test.sh`:
a gate with its own hermetic proof. Ruling: the gate gains a `selftest`
subcommand in the *same* script (no new file, no fixture tree, no env
override) that generates hostile fixtures in a mktemp dir at run time and
asserts each refusal fires, fail-closed if a check errors. The real gate
path is unchanged (no-arg invocation); CI's coverage job and the verify seat
gain one step apiece. Cost accepted over simplicity's position: the selftest
is permanent surface — but it reds loudly if it rots, and its fixtures are
generated, never shipped.

**R5 — Webhook sinks count every connection until their window closes; the
window's close is the assertion point. The cfg(test) budget seam is refused
as technically impossible.** The sinks fire their count channel at the
`expect`-th connection (`hits == expect` → `tx_count.take()`), so every
"4 attempts, no more" assertion resolves *before* the window it claims to
cover and a fifth attempt is unobservable. Adopted (simplicity's shape,
robustness's principle): delete the early fire in all three sinks; the count
channel resolves only at window close with the final observed count; the
window becomes a sink parameter (budget tests pass a window that outlives
the 2.1 s budget; the rest keep the existing constant); the 2800 ms sleeps
die — `rx_count.await` is the readiness signal; `expect` becomes a dead
parameter and goes away. Refused (robustness's preference): a `#[cfg(test)]`
seam shrinking the production retry budget to milliseconds — `tests/` suites
link the crate as a dependency *without* `cfg(test)`, so a test-only seam in
`src/` is invisible to them; the seam cannot work, and 0004 forbids mocks.
Cost of the refusal, accepted per robustness's own fallback: the budget
tests cost a few seconds of wall-clock per run. The can-fire leg: the
committed suite proves the count is *observed*, not echoed — a window
closing before the first attempt asserts the sink saw `0`, and the
full-budget windows assert `4` with the window outliving the budget (a
fifth attempt would red); a deliberately wrong budget (`seen == 3` against
the four-attempt budget) is demonstrated red in verify with captured
output, then reverted. No production change in the delivery path — no
decision document.

**R6 — Sleeps die by observable, not by seam: three additive gauges on the
existing `/metrics` registry.** The eleven sleep-as-sync sites all fake one
piece of knowledge — *the server registered my request* — which has no
client-side signal; a `#[cfg(test)]` seam cannot help (invisible to
`tests/`), and 0004 forbids mocks. Adopted: three small gauges, the run's
only `src/` delta, executed under the very tests that poll them:

- `swarmail_awaits_active` — waits currently parked in the wait machinery
  (REST await and MCP wait), maintained on every park/resolve path with a
  drop-guard so a panic cannot leak the count; covers the eight
  wait-park sites (lifecycle 315/346/382/408, api 579/614/641, mcp 1043);
- `swarmail_http_requests_in_flight` — in-flight HTTP requests (a small
  axum middleware counter); covers the seed-queue sites (lifecycle 556,
  binary 611): poll until the gauge reads N, then stop — all N requests
  have arrived;
- `swarmail_deliveries_in_flight` — SMTP deliveries between the DATA
  terminator and the insert resolving; covers the commit-burn site
  (lifecycle 451).

Tests poll deadline-bounded (the existing `poll_metric` pattern in
`tests/webhook.rs`) — an observed condition with a real deadline that fails
loudly, not a sleep wearing a timeout. The gauges are additive counters on
the registry the operators already read; no new endpoint, config, or test
hook; no newtypes or modules. Named consequence, not smuggled (robustness's
point, adopted verbatim): once the sleeps are honest, "stop under an
in-flight request" is exercised deterministically — if a request in flight
at stop turns out to panic rather than drain, that is a product bug the
sleep was hiding, named and fixed as its own change under 0024, never
re-slept.

**R7 — `http_json` fails loudly on a body that is not JSON.** Garbage
reading as `{}` makes every absence assertion pass against garbage. Ruling:
a non-JSON body panics naming the status and the body verbatim; a genuinely
empty body on a no-content status (204/304) stays absence — garbage and
absence are different failures and only the first is dishonest.

**R8 — The persist spin-gate releases on every exit path.** If the
`!task.is_finished()` assertion fails, `release` is never set and the
spin-gate thread spins forever — dropping the runtime hangs the runner
instead of failing the test. A drop-guard on `release` (the harness file
0020 sanctions, `src/store/persist_tests.rs`) makes red red.

**R9 — binary.rs reads its ports back from the child and kills its
children.** `swarmail serve` already logs the resolved bind addresses
(`src/main.rs`, the `Swarmail is up` line carrying `smtp = %…`,
`http = %…`) — the test throws that away with `Stdio::null()`. Ruling:
capture stderr, parse that one line for the resolved smtp/http addrs (ports
OS-assigned via `:0`), and add a kill-on-drop guard; the hardcoded
23456–23473 block dies. `sigint_and_wait` keeps its 30 s liveness deadline
and shape verbatim (ab19b9e: process exit is liveness, not the budget) —
the kill-on-drop replaces leaking, never the deadline. Zero production
change. Cost accepted: the test couples to the startup log line's format —
the binary's own stable statement, reding loudly at the exact line if it
changes.

**R10 — The 5k guarantee keeps its claim and gets its proof, non-required.**
`tests/rate.rs` is `#[ignore]`d while `docs/releases/v0.1.0.md` claims the
5k guarantee under "asserted in CI". Ruling: keep the claim; add a
release-profile CI job running `cargo test --release --test rate -- --ignored`,
tag-scoped (`v*` refs — the claim's own scope: the guarantee is proved at
the moment the release is cut), with an explicit `timeout-minutes`, in no
`needs:` list, and therefore not a required context — it cannot slow or
flake the required set, and a rare harmless red is the accepted cost.
Debug-mode correctness is already covered by the required matrix at every
push. Refused: editing the release notes to stop claiming it (the wrong
direction for a trust run) *and* shipping a flaky required job.

**R11 — The brokkr pin is recorded once, in the bundle, and CI consumes it
fail-closed.** Measured in this session: brokkr itself refuses unknown keys
at the bundle root (`error: bundle: recipe 'self' declares 'tooling' …
admits only …`), and it compiles a *sidecar file* cleanly — so the record
is `bundles/self/tooling.json` (version **and** pinned commit SHA; the
empirical skew — two 0.12.0 binaries, one refusing the bundle with
`'mcp' has unknown key 'carriage'` — is the argument against version-string
pins). CI's brokkr job reads the SHA from the record, checks out
feedback-loop-ai/brokkr at exactly that commit, builds, and asserts the
built `--version` equals the recorded version; a missing or malformed
record refuses with the path named — fail closed, never an unpinned
compile. Refused: the pin in `ci.yml` env (simplicity's preference) — it
would be a second copy of the same fact drifting against the bundle, and
the framing's own words bind the pin to the bundle; recording it in
bundle.json itself is impossible (brokkr refuses the key — measured). The
record is an operator edit (bundles/ is charter-pinned), written to
`.forge/run6-operator-edits.md`; until the operator applies it the job
stays red with a named refusal — the honest direction. The docker and
chart publish jobs `needs:` the brokkr job: a publish from a tree whose
constitution does not compile ships an unvalidated constitution. Cost
accepted: a brokkr build in the publish path (rust-cache keyed on the
brokkr checkout makes the steady state fast).

**R12 — Verify-seat parity is a recorded operator diff; the native-tls
check fails closed everywhere it runs.** The seat
(`bundles/self/scripts/verify-seat.sh`, charter-pinned) runs neither
`scripts/chart-verify.sh` nor `scripts/publish-guard-test.sh`. Ruling: the
exact intended diffs — add the hermetic `publish-guard-test.sh` run and the
gate selftest; document `chart-verify.sh` as CI-only (helm cannot be
fetched hermetically; a network fetch in a no-network seat is theater, not
parity); convert the native-tls check to fail-closed (`cargo tree -i
native-tls`: exit 1 = absent = pass, exit 0 = present = refuse, any other
exit = cargo failed = refuse) — go to `.forge/run6-operator-edits.md` with
one-paragraph rationales. In the tree (mine): CI's `deny` job gains the
same fail-closed native-tls step — the property is tree-wide and PRs hit
CI, not the seat.

**R13 — Both tag forms publish, and the guard probes both.** The docs
(README, release notes, index.html) say `:v0.1.0`; CI pushes `:0.1.0`; the
chart's default tag resolves to appVersion `0.1.0`. Ruling: the release
push writes **both** `:v<version>` and `:<version>` — the docs stay true,
existing `:0.1.0` pullers do not break, the chart stays coherent, and
decision 0019's chart-version rule is untouched (this is the image tag,
not the chart version). `scripts/publish-guard.sh` extends to probe *both*
manifests and refuse if either exists — that is not an amendment of 0019
but its conformance: 0019's "the registry is the only ledger" is only true
when the probe covers every tag the push writes (a `:v0.1.0` existing
while the guard checked only `:0.1.0` would overwrite it). The guard's
hermetic test gains the both-tag cases (404/404 pass; 200 on either
refuses; unknown refuses). Cost accepted: two manifest pushes per release.

**R14 — The deployment boundary extends by a proposed decision.**
Configurable NetworkPolicy ingress peers (default posture unchanged:
empty list renders today's `podSelector: {}`), a mountable TLS secret
wired to the binary's own `SWARMAIL_TLS_CERT`/`SWARMAIL_TLS_KEY` env
names, a startupProbe whose budget covers a full restore of the default
PVC, and the memory relation documented instead of invented (the store is
in-memory: limit ≥ restored state; `SWARMAIL_MAX_PER_INBOX` bounds it) —
these change the pod contract every install receives and a
security-relevant posture an operator must opt into knowingly, so they
carry [docs/decisions/0026-deployment-boundary-peers-tls-sizing.md](../../docs/decisions/0026-deployment-boundary-peers-tls-sizing.md)
(status *proposed*, awaiting the operator; amends 0008 by extension —
0008 §6's probe contract gains the startup probe, §2's "no configuration
surface" is untouched: peers, volumes and probes are deployment shape).
Decision: 0026. Refused: a conditional resource-override block (a silent
surprise better documented than coded), an invented memory default that
pretends 1 Gi of operator mail fits 256 Mi, an Egress section and
per-port peer matrices (none demanded — `policyTypes` stays Ingress-only),
and any change to restore-before-bind (0001-adjacent, not a knob).

**R15 — Supply-chain posture: pins by SHA, least privilege, honest
comments.** Every third-party action pinned to a commit SHA with a trailing
comment naming the tag it pins (git ls-remote at implement time;
dtolnay/rust-toolchain, Swatinem/rust-cache, taiki-e/install-action,
actions/*, azure/setup-helm, docker/*, rustsec/audit-check); top-level
`permissions: contents: read` with per-job grants exactly as needed (docker
and chart keep `packages: write`; dependency-audit keeps its checks/issues
grants); `rust:1-alpine` pinned by digest. Comment drift is a lying
citation: the "decision 0063" reference (a decision that does not exist) is
deleted — the truth is "Linux and macOS"; the coverage comment states what
the gate actually does (it resolves an installed nightly that carries
llvm-tools; nothing is auto-installed — no nightly is pinned by the
comment's claim today). Cost accepted: bumps become a deliberate
`git ls-remote` + one-line edit — that friction is the point.

## Acceptance Criteria

**#50 — deterministic, loophole-free gate**

- AC50.1: Repeated gate runs are exact — running
  `bash scripts/coverage-gate.sh` at the run's HEAD **three times** yields
  three exact 100/100/100 verdicts with identical counts, including under
  parallel load (the feed's `Closed` arm is driven deterministically by the
  bound-sender test; no retry logic exists in the gate).
- AC50.2: A red verdict names evidence under its own run-unique
  `target/coverage/run.*` directory (lcov + summary + log), and two
  concurrent gate runs each read only their own evidence — demonstrated by
  running two gates concurrently and observing distinct, self-consistent
  evidence paths.
- AC50.3: The declaration check refuses, with the file named: a `#` line
  merely containing `#[cfg(test)]` used as a guard; an unguarded
  `mod x_tests;` alongside a guarded copy in a comment or macro body; a
  file declared guarded in one parent and unguarded in another; a
  `#[path = "…"]` on a harness-named declaration; a harness-named file
  under `src/**/tests/`, `src/**/examples/` or `src/**/benches/`.
- AC50.4: `bash scripts/coverage-gate.sh selftest` generates hostile
  fixtures at run time and asserts every refusal arm fires (and that a
  well-formed tree passes the checks); it is red when a check regresses
  toward leniency and fail-closed when a check errors. CI's coverage job
  runs it.
- AC50.5: `coverage(off)` is detected in an **untracked** file under
  `src/` (plain grep, no git), and the check fails closed on grep error.

**#51 — tests that can fail**

- AC51.1: The counting sinks' channels resolve only at window close with
  the final observed count; the "4 attempts, no more" assertions hold with
  the window outliving the budget; the 2800 ms sleeps are gone; a window
  closing before any attempt observes and asserts `0`; the
  deliberately-wrong-budget variant (`seen == 3` under the four-attempt
  budget) is demonstrated red with captured output in verify, then
  reverted.
- AC51.2: `tests/binary.rs` spawns `swarmail serve` on OS-assigned ports
  read back from the child's own startup output, kills every child on
  drop (a failing test leaves no listener), keeps `sigint_and_wait`'s 30 s
  liveness shape, and the whole suite passes repeatedly with the port
  block deleted.
- AC51.3: All eleven named sleep-as-sync sites are replaced by
  deadline-bounded polls of the new gauges (`swarmail_awaits_active`,
  `swarmail_http_requests_in_flight`, `swarmail_deliveries_in_flight` on
  the existing `/metrics` registry); a poll that never observes the
  condition fails loudly at its deadline; the stop-refusal checks
  (lifecycle 451, api 614/641) are exercised with the request provably
  arrived, so they can no longer pass vacuously.
- AC51.4: `http_json` panics naming status and body on any non-JSON body;
  an empty body on a no-content status stays absence; the suite is green.
- AC51.5: A failed `!task.is_finished()` assertion in
  `src/store/persist_tests.rs` fails the test promptly (the spin-gate
  thread returns on every exit path) instead of hanging the runner.
- AC51.6: The 5k job exists, runs only on `v*` refs in release mode with
  `--ignored`, has an explicit timeout, appears in no `needs:` list, and
  is not a required context; the release notes' claim is unchanged.

**#52 — CI, seat and docs agree**

- AC52.1: CI compiles `bundles/self` with brokkr built from the commit SHA
  recorded in `bundles/self/tooling.json` (version + commit), asserts the
  built `--version` matches the record, and refuses with a named path when
  the record is absent or malformed; the docker and chart jobs `needs:` it.
- AC52.2: Every third-party action is pinned by commit SHA (comment naming
  the tag), top-level `permissions: contents: read` is present with
  per-job grants unchanged where granted, and `rust:1-alpine` is
  digest-pinned.
- AC52.3: CI's `deny` job runs the fail-closed native-tls check (absent =
  pass; present or cargo-error = refuse); the operator diff converts the
  verify seat's check the same way and adds the hermetic
  `publish-guard-test.sh` + selftest runs, with chart-verify documented
  CI-only in the seat script itself.
- AC52.4: No comment cites "decision 0063"; the coverage comment describes
  the gate's actual nightly resolution (resolved from installed
  toolchains, never auto-installed).

**#53 — the published contract matches the docs**

- AC53.1: A `v*`-ref release push publishes both `:v<version>` and
  `:<version>`; `publish-guard.sh` refuses when either tag exists and
  passes only when both are absent (its hermetic test covers both-tag
  404/200 cases); the documented `:v0.1.0` is the tag CI publishes.
- AC53.2: `networkPolicy.ingressPeers` renders each configured peer
  verbatim into `from:`; the empty default renders exactly today's
  `podSelector: {}` shape; `scripts/chart-verify.sh` asserts the locked
  default *and* that configured peers are admitted (both profiles).
- AC53.3: `tls.enabled` + `tls.secretName` mount the operator's Secret and
  set `SWARMAIL_TLS_CERT`/`SWARMAIL_TLS_KEY` to the mounted paths; unset
  values render no volume, no mount, and no TLS env (plaintext listener
  byte-identical); chart-verify asserts the wiring, never cert lifetime.
- AC53.4: The pod carries a startupProbe on `/healthz` whose budget is
  documented against a full restore of the default PVC, and the memory
  relation (limit ≥ restored state; `SWARMAIL_MAX_PER_INBOX` bounds it) is
  documented beside `resources` — no conditional override, no invented
  default claim; restore-before-bind is unchanged.
- AC53.5: `.dockerignore` excludes `.forge/` and `*.db`; NOTES.txt names
  the generated claim only when it is generated; Chart.yaml's license
  annotation reads `MIT OR Apache-2.0`.

**Standing gates** — every house gate clean at the run's HEAD:
`cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`,
`cargo test --workspace --locked`, `cargo deny check licenses`,
`scripts/coverage-gate.sh` (exact 100/100/100, three times),
`brokkr compile --bundle bundles/self`. No `coverage(off)`, no coverage
floor movement, no new dependency, no edit to `agents/`, `adapters/` or
`bundles/` (operator edits go to `.forge/run6-operator-edits.md`), no
issue closed, no PR opened.
