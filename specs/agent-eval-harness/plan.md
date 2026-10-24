# Implementation plan — agent eval harness

Implements [spec.md](spec.md) and proposed
[0055](../../docs/decisions/0055-agent-eval-harness.md). This is the design
handoff, not a receipt for implemented functionality.

## Technical context

Standalone Rust crate, one Unix executable, Tokio processes/networking,
Axum HTTP, Serde JSON, reqwest with default features disabled, minimal
plaintext SMTP and fixed MIME templates. Use only necessary versions already
present in root Cargo.lock (including transitives); libc may supply Unix
group signals, tempfile owns scratch space, base64 encodes MIME/AUTH, and
existing random/time/CLI crates may be reused. Pin exact direct versions
and generate the lock with `--offline`; validate the entire closure.
No root path dependency and no root workspace membership.

There is no plan template in this checkout; these sections follow the
required spec-kit structure. Research, data model, contracts and quickstart
live here rather than creating additional design-commit files.

## Constitution check

- 0001, 0002, 0003 and 0005 remain intact. Real SMTP followed by MCP consumes
  their behavior without changing or duplicating the store or extractor.
- AUTH username names the inbox (0004); isolation is local cooperation,
  not access control. Existing loopback Host/Origin policy (0043) is obeyed.
- Apply 0020's deterministic-test discipline: generated dates, explicit
  barriers, chaos at 0/100, no sleep-based validity/readiness assertions,
  per-trial state and real protocol acceptance tests.
- The standalone ruling explicitly excludes evals from root measurement.
  No coverage attributes, denominator filtering or gate edits. Root exact
  100/100/100 remains literal missed == 0.
- Until the follow-up after #120 batch 5, **evals is proven by its own suite
  run by the implement seat and reproduced by reviewers**. Root green does
  not prove evals. Keep this in delivery notes and proposed 0055.
- This design commit contains exactly these three spec files and proposed
  0055. No decision-index or historical decision edits belong to this run.

## Research and resolved questions

Both `.brokkr/state/design/positions/{simplicity,robustness}.md` and all
four issue files were read. `.brokkr/state/tasks/` is absent. The original
`runA-intake-frame.json` documents a blocked workspace contradiction;
`runA-intake-frame-r2.json` and the operator's later task ruling resolve it
with a standalone workspace. Do not revive the original root-test requirement.

`tools/coverage-fn-scan/Cargo.toml` demonstrates the independent workspace.
Root Cargo.toml's explicit publication include list needs no edit.
`src/main.rs` announces actual smtp/pop3/http ports; reuse the format parsed
by `tests/binary.rs::parse_addrs`, without importing root test helpers.
`src/config.rs` accepts port zero and otherwise has environment-sourced
settings that must be removed or explicitly pinned for child startup.

`src/mcp.rs` supplies initialize, search/get/wait and chaos tools. Waits cap
at 60000 ms and successful tool data is JSON inside text content. There is
no MCP attachment retrieval. ICS download is therefore an explicit fixture
affordance, not a fallback for a defective tool. Support's public HTTP
reply avoids an unanswerable oracle task without implementing #160.
Only 0055 is available to this run; 0049–0054 are reserved for #120.

## Project structure and files

| Planned path | Responsibility |
|---|---|
| `evals/Cargo.toml`, `evals/Cargo.lock` | Standalone unpublished package and offline subset dependency graph. |
| `evals/src/main.rs` | CLI dispatch, single stdout owner and exit conventions. |
| `evals/src/lib.rs` | Shared typed task/check/result interfaces for executable and tests. |
| `evals/src/fixture.rs` | Public/admin routers, identity/flow state, redacted snapshots and explicit scenario barriers. |
| `evals/src/mail.rs` | Bounded SMTP/AUTH client and fixed MIME/ICS generation. |
| `evals/src/tasks.rs` | Fixed 16-task catalogue, strict rendering, pure predicates and failure enum. |
| `evals/src/mcp.rs`, `evals/src/oracle.rs` | Current MCP protocol and public per-task scripted solutions. |
| `evals/src/runner.rs` | Binary resolution, owned trial processes, environment, snapshots, cleanup and results. |
| `evals/src/report.rs` | Matrix validation/aggregation and report math. |
| `evals/tests/proof.rs` | Every oracle/null task and two concurrent isolation trials through the executable. |
| `evals/tests/fixture.rs`, `evals/tests/runner.rs`, `evals/tests/report.rs` | Wire refusal controls, external-command lifecycle and deterministic report proofs. |
| `evals/README.md` | Runnable examples, public fixture routes, agent contract, taxonomy, limits and interim gates. |

These ownership boundaries may be combined when small, but proof target
`proof` and public CLI/schema stay stable. No file outside evals changes in
implementation except repairs to these four design artifacts if required.

## Data model

`Task` owns versioned id/tier/target failure/idle class/template/scenario/
predicate/agent count/deadline. A catalogue validator runs before any server
or child starts; `tasks` serializes that same catalogue.

`FixtureState` owns a trial ID, identity-to-inbox mapping, per-flow states,
random secrets with `(identity, flow, generation, role)` provenance,
delivery attempts, ordered categorized rejection events and completion
facts. Secrets are unique within a trial; generated OTPs cannot equal a
superseded generation. An OTP carries an expiry checked at submission;
fixture-local unit tests inject clock instants, never root seams.
Passwords are synthetic, kept private and excluded from snapshots/logs.

`StateSnapshot { schema_version, trial_id, task, identities, flows,
deliveries, events }` is obtained atomically after freezing fixture
mutations/timers on the admin plane. State read endpoints return immutable
redacted views. Snapshot serialization is distinct from private state
serialization so adding a secret field cannot accidentally expose it.
Per-flow predicates demand validated completion facts and the appropriate
delivery receipts. No supplied `success` field is a check input.

`TrialResult` is the exact spec schema. Lifecycle failures and deadlines
wrap the pure business check; they do not make the scorer read agent
accounts. Ordered `agent_exits` retain each actual subprocess status for
isolation. Artifact metadata records the binary path/version and harness
revision for later reproduction; it is not model telemetry.

## Contracts and implementation sequence

1. Establish package, lock closure validation and typed contracts first.
   Define `trial`, `tasks`, `check`, `matrix` and `report` CLI parsers and
   schema tests. Use literal argv arrays for matrix agent specifications.
   Validate positive deadlines/counts and reject duplicate agent labels.
2. Implement fixture HTTP routes as documented JSON operations. Public
   flow-start routes and emailed capability routes perform the real
   transitions; a per-flow admin GET supplies redacted facts. Admin setup,
   freeze and scenario-release operations are separate from read-only GETs.
   No public setup route mints a success fact or returns a mail secret.
3. Write SMTP/MIME before oracle logic. Test live DATA acceptance and MCP
   visibility for each template. Verify MIME's ICS bytes independently in
   the mail-builder tests and compare them to the authorized app download;
   never use REST retrieval in an oracle. Unexpected loss after 250 is a
   reported Swarmail defect, not permission to poll another transport.
4. Implement the runner owner with a bounded provisioning deadline (default
   30 s), default agent deadline (60 s; explicit positive overrides),
   network timeouts and a bounded shutdown grace (5 s then group kill).
   Resolve/build root once before batches; integration proof setup uses
   `OnceLock` or equivalent and explicit `SWARMAIL_BIN` for child trials.
   Use Cargo JSON artifact output to locate the built executable, respecting
   target-directory configuration rather than guessing `target/debug`.
5. Spawn each service/agent into an owned Unix process group. Use
   kill-on-drop as a fallback, but explicitly signal groups, await children
   and drain logs on all handled exits. Stop leftover descendants even when
   their leader exits normally. Cap retained logs while continuing to read
   pipes so chatty children cannot deadlock. Test signal, spawn and server
   failure paths using real processes. Error reporting never shares stdout
   with a child. No detaching support or hostile-child confinement claim.
6. Implement scenario provisioning with SMTP receipts and barriers. Runner
   controls 100% chaos then clear/retry before launch. Agent controls the
   documented slow-mail release after its initial wait. Multi-agent setup
   completes before a common launch barrier; the proof uses observable
   readiness to keep both isolation trials active concurrently.
7. Implement oracle/null as child subcommands using only exported public
   inputs. Oracle initializes MCP, waits/searches within its inbox, unwraps
   current text JSON and takes normal HTTP actions. It chooses by goal,
   identity and mail content, not hard-coded secret values. Null exits zero
   without actions. Test helper commands supply trap and never-exit behavior;
   there is no third public built-in or generic adversarial-agent framework.
8. Freeze, capture and validate snapshot, clean up, run shared predicate,
   write optional artifacts atomically, emit one row. A supervisor catches
   recoverable errors/panics and handles SIGINT/SIGTERM. Broken stdout and
   uncatchable termination are explicit limits, not fabricated guarantees.
9. Add matrix/report and public documentation after controls pass. Matrix
   executes exactly the requested product and reports all attempted rows.
   Unit math fixtures cover Wilson endpoints and nearest-rank quantiles;
   CLI tests prove decoding and output behavior. No baseline/model run here.

## Quickstart and validation

The implementation README must make this sequence runnable from the repo
root (the design does not assert these commands already exist):

```bash
cargo build --offline --locked --bin swarmail
cargo build --offline --locked --manifest-path evals/Cargo.toml
export SWARMAIL_BIN="$PWD/target/debug/swarmail"
evals/target/debug/swarmail-evals tasks
evals/target/debug/swarmail-evals trial --task signup --agent oracle
evals/target/debug/swarmail-evals trial --task signup --agent null
evals/target/debug/swarmail-evals trial --task signup --agent custom -- /absolute/agent --mode email
cargo test --manifest-path evals/Cargo.toml --locked --test proof
```

The default-target example above must note custom `CARGO_TARGET_DIR` paths;
the runner's automatic binary discovery uses Cargo artifacts. README also
supplies concrete matrix agent-JSON and report commands and `check` against
retained `state.json`. External/model commands are examples, not run-A gates.

From evals run `cargo fmt --all -- --check`,
`cargo clippy --all-targets --locked -- -D warnings`, and
`cargo test --locked`. Prove the dependency closure offline, license policy
and absence of native-tls in evals too. From the root run, in order,
`cargo fmt --all` before each check:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
bash scripts/coverage-gate.sh
bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh
cargo deny check licenses
```

Do not override TMPDIR. Verify also runs the existing fail-closed native-tls
absence proof and tooling-pinned `brokkr compile --bundle bundles/self`.
This seat must record any sandbox gate failure honestly; it must not install
tools, change gates or call an incomplete proof green. If cargo itself is
refused, stop and preserve `.brokkr/state/epic164-harness.patch`; if signing
is impossible, stop rather than committing unsigned. Design validation
checks required headings, nonempty artifacts, task checkboxes, proposed
status and relative links; it cannot prove the future implementation.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Green root tests conceal broken evals. | Explicit separate suite, implement receipt and reviewer reproduction; follow-up CI/verify wiring after #120 batch 5. |
| Lock resolution adds an uncached/transitive package. | Offline generation, exact direct pins and full name/version/source comparison to root lock; refuse rather than widen allowlist. |
| Oracle only proves an internal fast path. | Execute both built-ins through the identical external child seam and real CLI. |
| Fixture leaks answers or accepts arbitrary success. | Separate bearer admin listener, redacted types, token/identity validation and negative wire controls. Unconfined same-user access remains possible and disclosed. |
| MCP attachment/reply gaps make oracle proof impossible. | Explicit fixture ICS download and HTTP challenge reply; no claim to measure #159/#160 and no silent transport fallback. |
| Timer scheduling changes trap presence. | Superseded OTP, controlled slow-mail release, pre-agent observed chaos refusal/retry; do not infer waits from state. |
| Taxonomy invents agent reasoning. | Target versus idle declaration, app provenance events, fixed precedence and focused negative controls; unknown external MCP misuse remains unobservable. |
| Port/build races or cross-trial state. | Port-zero binds, fresh processes/temp dirs, once-per-proof binary resolution and two concurrently active isolation trials. |
| Child output or descendants hang shutdown. | Continuous bounded-retention drains, group ownership, grace/kill/reap and failure-path process tests; no guarantee for daemonized or hostile children. |
| Agent crashes after achieving goal. | Preserve raw exit status; state success wins unless infrastructure fails or deadline expires, with tests for all precedences. |
| Scoring state changes during capture. | Freeze mutations and delivery timers, take one consistent snapshot, then stop services. |
| A real Swarmail defect appears. | Preserve minimal SMTP/MCP reproduction and report outstanding acceptance; no `src/` edit or workaround. |
| Reports hide infrastructure errors or overclaim sample size. | Print all attempts, error rate, n, both denominators, defined Wilson/quantile conventions and null empty estimates. |


## 2026-10-24 clarification — decision 0075 (renumbered from 0062) (proposed)

The earlier plan's inherited environment and bounded raw-log retention are
superseded by [eval privacy](../eval-privacy-port/spec.md). Trial and matrix child
commands use exact closed role maps, private `/tmp` runtime directories,
caller-boundary executable resolution and explicit wrapper authentication.
Cargo preparation uses its own cleared, named toolchain map. All output streams
are drained but arbitrary bytes are discarded; typed service announcements,
Cargo records, matrix rows and safe diagnostics use the bounds and failure
rules in decision 0075 (renumbered from 0062). Metadata no longer records an executable path. Result
schema, scoring, lifecycle, real protocol proofs and honest `unconfined`
reporting remain unchanged.

## 2026-10-24 integration onto matrix cancellation

Proposed [0075](../../docs/decisions/0075-eval-privacy.md), renumbered from
0062, and [the port contract](../eval-privacy-port/spec.md) govern the current
closed environments and safe publication. Preserve #194's owned waiters,
pre-spawn signal handlers, concurrent five-second TERM grace plus five-second
post-KILL observation, final cleanup fence and complete matrix accounting.
Private runtime owners survive abort, reaping and drainage. Agents receive
generated PWD equal to actual cwd. Matrix row capture remains 64 KiB. Artifact
publication is exclusive, initially private, descriptor-normalized before
payload and atomic per file with no clobbering. Existing entries remain intact.
Historical receipts above are not proof of this port. Five fresh 0058 careful
reviews and separately identified Linux/macOS evidence remain obligations.
