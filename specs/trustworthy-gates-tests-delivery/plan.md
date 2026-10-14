# Plan — trustworthy gates, tests and delivery (#50–#53)

## Technical context

One Rust crate (`src/`), real-protocol integration suites under `tests/`
(0004: no mocks), a bash+awk+jq coverage gate
(`scripts/coverage-gate.sh`) enforcing literal 100/100/100, CI in
`.github/workflows/ci.yml`, the Helm chart under `charts/swarmail`
verified offline by `scripts/chart-verify.sh`, and a hermetic verify seat
(`bundles/self/scripts/verify-seat.sh`, charter-pinned — operator edits
only). The delivery constitution compiles via
`brokkr compile --bundle bundles/self`; the workspace brokkr lives at
`.forge/bin/brokkr` (its stderr `REVIEW-RESIDUAL-OK` note is benign).
Git in harness sessions needs `GIT_CONFIG_COUNT=0` (the same workaround
the gate drops with its `git grep`).

Measured anchors (main has moved since ac8c326 — locate by shape):

- `src/feed.rs` — the two-arm `tokio::select!` (`recv` vs
  `shutdown.changed()`) and the inline test passing `quiet_shutdown().1`.
- `scripts/coverage-gate.sh` — shared evidence paths
  (`target/coverage/lcov.info` read by every stage after the `cp`), the
  attribute-stack awk with first-match `break`, the `git grep`
  `coverage(off)` check, no `#[path]` or harness-dir refusal.
- `tests/webhook.rs` — three counting sinks firing `tx_count` at
  `hits == expect`, the 2800 ms sleeps, `poll_metric`'s deadline-bounded
  pattern to copy.
- `tests/binary.rs` — the 23456–23473 block, `Stdio::null()` on the
  child's stderr, `sigint_and_wait` (30 s liveness, keep).
- `src/main.rs` — the `Swarmail is up` line already carrying
  `smtp = %…, http = %…` resolved addrs.
- `src/api.rs` — `await_messages`/wait handlers, the `/metrics` handler's
  `rows` array (the append point), the router layer site for middleware.
- `src/smtp.rs` — the DATA path where the delivery's insert resolves.
- `src/store/persist_tests.rs` — the `set_pre_mirror_gate` closure spinning
  on `rel`, released only after the assert.
- `.github/workflows/ci.yml` — no top-level `permissions:`, floating
  action tags, the nonexistent "decision 0063", the nightly comment drift,
  the release push writing `:${version}` only.
- `scripts/publish-guard.sh` — probes `/manifests/${version}` (one form);
  `scripts/publish-guard-test.sh` — hermetic python3 localhost fixture.
- `charts/swarmail/` — `values.yaml` (resources, probes, networkPolicy),
  `templates/networkpolicy.yaml` (`podSelector: {}` under `from:`),
  `templates/deployment.yaml` (liveness/readiness on `/healthz`), NOTES.txt
  (generated claim), Chart.yaml (license annotation `MIT`).
- `.dockerignore` — lacks `.forge/` and `*.db`.
- `Dockerfile` — `FROM rust:1-alpine` (unpinned).

## Constitution check

- **Frozen guarantees** — 0001/0002/0003/0005 untouched: no `Store::insert`
  path, no await ordering, no pruning, no extraction change. The closest
  approach is `swarmail_deliveries_in_flight` around the SMTP insert *call*
  and the axum in-flight middleware — additive counters outside the
  mutation boundary's semantics; 0024's stop-refusal contract is *exercised
  more*, never weakened.
- **0020 disciplines** — the feed fix keeps the test inline (rule 1,
  branch-stable); the persist drop-guard stays in the sanctioned harness
  file; no wall-clock validity assertions anywhere; no sleep as
  synchronization (the gauge polls are observed conditions with real
  deadlines); chaos untouched (poles only); harness names stay reserved
  (the strengthened declaration check enforces harder).
- **0004** — no mocks: the readiness observables are the production
  server's own `/metrics` rows, read over real HTTP.
- **0010** — the exact-count asserts in lifecycle/api stay exact; the
  gauge conversions replace only *sequencing* sleeps, never relax a count.
- **0019** — ref-bind and fail-closed probe preserved; the probe extends
  to cover every tag the push writes (the ruling's own "registry is the
  only ledger").
- **0018 §6 / frozen contexts** — the five required check names keep their
  names and meanings; new jobs (`brokkr-compile`, the 5k release job) are
  additions; publish jobs gain `brokkr-compile` in `needs:`.
- **Decision: 0026** (proposed) — [docs/decisions/0026-deployment-boundary-peers-tls-sizing.md](../../docs/decisions/0026-deployment-boundary-peers-tls-sizing.md)
  covers the chart's deployment-boundary extension; acceptance is the
  operator's.
- **Gates** — every new production line (three gauges, middleware, counter
  plumbing) ships with the tests that execute it; the exact gate stays
  100/100/100 with no exclusions; `cargo deny` allowlist untouched (no new
  dependencies anywhere in this plan).

## Project structure

Files touched by this run (all existing; no new committed file):

- `scripts/coverage-gate.sh` — per-run evidence dir; checks factored into
  functions taking the source root (the real path passes `src`); hardened
  declaration check; `#[path]` + harness-dir refusals; plain-grep
  `coverage(off)`; `selftest` subcommand.
- `src/feed.rs` — one line in the inline test (bind the sender).
- `src/api.rs` — `swarmail_awaits_active` maintenance on the wait paths,
  `swarmail_http_requests_in_flight` middleware, three new `/metrics` rows.
- `src/smtp.rs` — `swarmail_deliveries_in_flight` around the delivery
  insert (drop-guarded).
- `tests/webhook.rs` — sinks count until window close; window parameter;
  sleeps die; observed-count and full-budget assertions.
- `tests/binary.rs` — stderr capture + startup-line parse; OS-assigned
  ports; kill-on-drop guard; sleeps → gauge polls.
- `tests/lifecycle.rs`, `tests/api.rs`, `tests/mcp.rs` — sleeps →
  deadline-bounded gauge polls.
- `tests/common/mod.rs` — `http_json` fails loud on non-JSON.
- `src/store/persist_tests.rs` — release drop-guard.
- `tests/rate.rs` — unchanged (the job runs it as-is).
- `.github/workflows/ci.yml` — permissions, SHA pins, brokkr-compile job,
  5k release job, native-tls step, both-tag release push, selftest step,
  comment repairs.
- `scripts/publish-guard.sh` + `scripts/publish-guard-test.sh` — probe
  both tag forms + tests.
- `Dockerfile` — digest pin.
- `charts/swarmail/values.yaml`, `templates/networkpolicy.yaml`,
  `templates/deployment.yaml`, `templates/NOTES.txt`, `Chart.yaml`.
- `scripts/chart-verify.sh` — peer/TLS/probe assertions, both profiles.
- `.dockerignore` — `.forge/`, `*.db`.
- README/docs — only where a sentence must match reality (e.g. the dual
  tag note); the release notes' 5k claim stays.

Run-local (uncommitted): `.forge/run6-operator-edits.md` — the
verify-seat.sh diff (publish-guard-test + selftest + documented
chart-verify delta + fail-closed native-tls) and the
`bundles/self/tooling.json` record, each with a one-paragraph rationale.

## Approach

**#50 (gate).** Create `target/coverage`, then `mktemp -d
target/coverage/run.XXXXXX` as the evidence dir (not trapped); point
`--output-path`, the leak check, the declaration check, the counting awk
and the summary `jq` at it; name it in every refusal; keep the build dir
trapped as today. Factor the three source-tree checks into functions
`check_no_coverage_off <dir>`, `check_harness_declarations <dir>`,
`check_no_harness_dirs <dir>`; the main path calls them with `src`.
Rewrite the declaration awk: collect every `mod <stem>;` declaration line
per parent (declaration = the line *is* the declaration), walk the
contiguous attribute stack above each (blank/comment lines skipped, a
non-attribute line breaks), require the trimmed line `#[cfg(test)]`
exactly once in that stack; refuse if any parent declares the stem
unguarded; refuse any `#[path =` attribute on a harness-named declaration
line; find-based refusal of `src/**/{tests,examples,benches}` directories
containing files, fail-closed on find error. Replace the git grep with
`grep -rn 'coverage(off)' src/ --include='*.rs'` (status ≥ 2 refuses).
Add the `selftest` branch: mktemp fixture tree, one hostile fixture per
refusal arm (doc-string cfg guard; unguarded decl + commented guarded
copy; two parents, one unguarded; `#[path]` escape; harness dir under
src; untracked `coverage(off)`), assert each check refuses and a clean
copy passes; any unexpected status refuses. Feed fix: bind the sender in
`a_cleared_inbox_ends_the_feed_after_the_snapshot`.

**#51 (tests).** Sinks: parameterize the window, delete the early fire,
resolve the count at window close, delete the sleeps, drop `expect`;
assertions become `let seen = rx_count.await`. Add the observed-count leg
(window shorter than the first attempt ⇒ `0`). binary.rs: spawn with
`:0`, pipe stderr, parse the `Swarmail is up` line for `smtp =`/`http =`
addrs, wrap the child in a kill-on-drop guard, delete the port block;
`sigint_and_wait` untouched. Gauges: add the three counters with
drop-guarded inc/dec, expose them in the `metrics` handler's `rows`, and
convert the eleven sleep sites to a shared deadline-bounded
`wait_for_gauge(addr, name, expected)` helper in `tests/common/mod.rs`
(poll `/metrics`, small backoff, fail loudly at the deadline). http_json:
parse the body, panic with status + body on parse failure unless the
body is empty and the status is 204/304. persist_tests: drop-guard on
`release`.

**#52 (CI/seat/docs).** Add top-level `permissions: contents: read`;
per-job grants where already granted. Pin every action via
`git ls-remote <repo> <tag>` → SHA + trailing `# <tag>` comment; pin
`rust:1-alpine` by digest (`docker buildx imagetools inspect rust:1-alpine`
or the registry API). New `brokkr-compile` job: jq-read
`bundles/self/tooling.json` (refuse named when absent/malformed), checkout
feedback-loop-ai/brokkr at the recorded SHA, `cargo build --release`,
assert `--version` equals the recorded version, then
`brokkr compile --bundle bundles/self`; docker + chart add it to
`needs:`. New `rate-5k-release` job: `if: startsWith(github.ref,
'refs/tags/v')`, `cargo test --release --test rate -- --ignored
--nocapture`, `timeout-minutes: 15`, in no `needs:`. `deny` job gains the
fail-closed native-tls step. Release push tags both forms; guard probes
both manifests (404/404 pass; 200 either refuses; anything else refuses)
and its test gains the cases. Fix the two comments. Write the operator
diffs to `.forge/run6-operator-edits.md`.

**#53 (delivery).** values: `networkPolicy.ingressPeers: []`,
`tls.enabled: false`, `tls.secretName: ""`, startup-probe budget keys,
commented memory relation. networkpolicy.yaml: empty peers ⇒ the exact
today `from: [- podSelector: {}]`; non-empty ⇒ render each peer verbatim.
deployment.yaml: mount the Secret when tls set, env `SWARMAIL_TLS_CERT`/
`SWARMAIL_TLS_KEY` to the mounted paths, add the startupProbe on
`/healthz`/http. NOTES.txt: claim only when generated. Chart.yaml:
license annotation. chart-verify.sh: full profile sets peers + tls and
asserts the rendered admission, mount and env; default profile asserts
the locked shape and no TLS env; the probe budget is asserted present.
ci.yml: push both tags. .dockerignore: two lines.

## Verification order (the run's own)

1. Red proofs first, output captured: plant each hostile fixture against
   the real checks (then the permanent selftest carries them); the
   deliberately-wrong-budget sink assertion; chart-verify with a dropped
   peer assertion.
2. Revert the plants; run every house gate; the coverage gate **three
   times**, exact; two concurrent gate runs with distinct evidence paths;
   `brokkr compile --bundle bundles/self`; `cargo deny check licenses`.
3. Confirm `.forge/` and `*.db` are out of `docker build` context
   (`.dockerignore` is consulted by buildkit — assert with
   `docker build --check`-style dry inspection or the CI build).

## Risks

- **The gate's hardened awk misparses a legitimate shape** (a real
  attribute style the tree uses but the walker rejects) → the gate reds
  loudly naming the file; the fix is in the walker, not a weakening.
  Mitigation: run the full suite + the selftest on every edit; the
  tree's own `src/store/persist_tests.rs` declaration is the canary.
- **The startup-line parse couples `tests/binary.rs` to a log format** →
  a format change reds the test at the exact line. Mitigation: parse the
  structured fields (`smtp = …`, `http = …`) tolerantly; the coupling is
  the binary's own stable statement.
- **Gauge drift** (an inc without its dec on a panic path) → a stuck
  count breaks the polling tests loudly (deadline), never silently.
  Mitigation: drop-guards on every inc; the polls' deadlines are the
  detector.
- **The seed/commit tests could race stop between arrival and boundary
  entry** → the test would exercise 0024's gate refusal instead of the
  queue/burn path. Mitigation: the asserted invariants (no 250, stop not
  held) hold under *every* interleaving from arrival; 0010 exact counts
  stay; the burn path is additionally covered by 0012's own tests.
- **A brokkr build from SHA is slow or the SHA vanishes (force-push)** →
  the job reds with a named refusal; rust-cache keys the checkout;
  `rev-parse`-vs-record catches a rewritten history. Cost accepted: a
  deliberate re-pin is the recovery.
- **Dual tags publish two manifests** → the guard's probe covers both;
  the recovery for a bad publish stays "delete the registry tag"
  (0019's own sentence).
- **The 5k job flakes on a loaded runner** → non-required, timeout-
  bounded, tag-scoped: a red there delays nothing and blocks nothing.
- **Helm unavailable to a seat running chart-verify locally** →
  chart-verify stays CI-only for the seat (documented operator diff);
  in-tree CI owns the assertion; the workspace `.forge/bin/helm` exists
  for local runs but nothing hermetic depends on it.
- **Operator edits not yet applied** → the brokkr job refuses with the
  named path (fail closed) until the operator lands
  `bundles/self/tooling.json`; the rest of CI stays green and the
  refusal names the fix.
