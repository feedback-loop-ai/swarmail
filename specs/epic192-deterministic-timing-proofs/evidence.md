# NEXT-177 implementation evidence

Date: 2026-10-23. Scope: all six #177 findings, the additional MCP cap elapsed
check, and both sibling reaping proofs. This records the implementer phase;
hermetic verify, the four independent reviews and ship remain engine work.

## Provenance and scope

Candidate base is `b576466d055592d5b143848250b10f55106120a6`; chief's design is
`fa62e66`. The engine's deterministic design result is
`.forge/results/bd4dee76-c7dd-4872-aad2-eaa2763e1096-check.json`; the implementer
also ran `bash bundles/self/scripts/design-check.sh check .` successfully
before implementation. Intake classified `engine` in
`.forge/results/b34142f7-1e30-4095-9074-81060ed6656b.json`.

All writes and commands below used the brokkr workspace tool. The inherited
CPU affinity was 96–111; Cargo used this checkout's private `target/`.
No new run, agent, push, merge, tag or publication was launched. No G patch
or private helper was imported. No dependency, gate, policy, dispatch pin,
Store/Persist mutation implementation or accepted ruling changed.
`tests/common/mod.rs` is byte-identical to intake after temporary controls.

The historical directory `.forge/epic192/actual-pop3-coverage-flake` is
byte-identical to its initial whole-directory manifest. Its host result is
still a failure: 12446/12444 lines, 724/723 branches, 1379/1379 functions,
with POP3 1334/1335 and the false outcome at 1332 missed. LCOV SHA-256 is
`fe57f46356bf0a4f4350b640df033015d3459d57cfa04c6db6c8d03869bcd43a`.
New greens do not change that receipt. Shared counter pre-credit is proved
to be an unsound observation class by source and controls; the historical
execution's exact schedule and cause remain unproven.

Evidence root, relative to this checkout:
`.forge/epic192/next177-evidence/`. `baseline.json`,
`receipt-unchanged.json`, `final-source-hashes.json`, `controls-ledger.json`
and `gates.json` bind the receipts to source identities. These run-local
files and `target/coverage/` are preserved, gitignored evidence.

## Finding-by-finding disposition

| Finding | Final proof and actual execution |
|---|---|
| 9.1 REST/MCP malformed, cap and zero | Exclusive inbox cumulative registration rows are established by legal zero waits. Refusals have zero delta; zero waits increment once and leave no active wait. Existing wire envelopes, exact refusal text and seeded MCP non-consumption remain. `zero_timeout_finishes_on_the_first_router_poll` and `zero_timeout_finishes_on_the_first_dispatch_poll` each run once: zero is Ready with the required result; omission and positive timeout are Pending, resolved by controlled stop. Full API 64/64 and MCP 43/43 pass. |
| 9.2 stdio | Complete request observation and explicit socket release replace the five-second sleep. Id 11, -32000 and exact `read timeout` are checked; peer and bridge work are collected with diagnostic bounds and peer abort/join cleanup. Full stdio 31/31 passes. Closing-peer and lost-event controls fail their intended assertions, then restore green. |
| 9.3 DATA helper | Scripted real dialogue reaches DATA and withholds 354 on a release event. The client sends no body, and its own exact named 30-second read-timeout panic is required. An outer diagnostic timeout is failure. Full smtp_edges 58/58 passes; bypassing the actual common read timeout fails the DATA mechanism assertion. Common helper defaults and diagnostics are unchanged. |
| 9.4 SMTP/POP3 backoff | Each actual serve captures its own probe. Its real backoff operand reports Pending on an installed readiness latch while a greeted task is live. Stop finishes, joins the owned tasks and closes the socket before latch release. A second case releases the latch and exercises retry and the ordinary owner-created timer. Both protocol tests pass; stacking stop behind the held operand fails both named stop-before-release diagnostics. No elapsed ceiling remains. |
| 9.5 since_ms | First seed's actual timestamp/id supply equality and checked +1 boundaries across list, await and assert, with exact identities/counts and malformed refusals retained. The separate count=2/limit=1 request omits since_ms, so tied/regressing parse-start stamps are allowed. The API suite and full workspace gate pass. |
| 9.6 admitted persistence | Existing HeldCommit entry precedes stop; its release flag is explicitly closed after stop. No 250, live/disk/counter/fanout effect occurs while held; release produces the existing exact once commit and an additional no-second-notification assertion. STOP_BUDGET remains a diagnostic fence. The original test passes, and premature DATA acknowledgement fails `an unfinished commit was acknowledged`. |
| Reaping, both protocols | REAPED is removed. Task-local scope is captured once per serve; only its actual JoinSet harvest increments its watch. One owner has two batches (one completion, then two), exact counts 1 and 3, actual AbortHandle completion while accept is parked, first-offer admission and full-cap refusal. A second real owner cannot contribute credit. Each same pinned watch wait is explicitly Pending before completion and Ready afterward. Both inline tests pass; four harvest/isolation controls fail both protocol instances. All replacement lines/branches/functions are measured. |

Zero/default, valid/refused and timestamp equality/+1 are paired public-entry
or input controls. They are not claimed as parser/filter source mutations;
those owners are outside this lane. Production acceptance/commit visibility,
subscribe-before-check, oldest-only pruning, ingest extraction, one shutdown
watch and every runtime budget remain unchanged. Observation seams are
cfg(test)-only and do not ship or reap for the owner.

## Final source identities

| Source | SHA-256 |
|---|---|
| `tests/api.rs` | `146e95578f857d7cc3c24d9c7f020eb53bab483fbe477556520f1f9e1a1ab321` |
| `tests/mcp.rs` | `d222446f9f801f9a3038ccef310d7a6e2932e3b7fd9e00d70df9a6eea8fffa3e` |
| `tests/stdio.rs` | `6f374aefd5fd1c18fd799342a0861045c98699f628bcd623d788a7f37549509f` |
| `tests/smtp_edges.rs` | `6bb5191b9025ad59db3ca9c455f63a871e29aaa060f9224adf41c31063333e68` |
| `tests/common/mod.rs` | `4f24e2dfd54d8a56d816c473d43eb353169fbfeb9e27b6f5b0f1012f88aeb5b2` |
| `src/smtp.rs` | `841aa12d2777a3b597658086f54c1aa0b4032b62675753a08a28d354f5f1da55` |
| `src/pop3.rs` | `5ff31ea302f6a16df054e02d6d7e48ab92ed4b02ba614e8cd50fff5d12ed48f2` |
| `src/store/persist_tests.rs` | `92030268ccea958a2648365b6096c5dd45d8385d86d8ffbe888f41e754dad533` |

## Actual red/restored controls

Every listed control compiled, failed the intended behavioral assertion,
restored exact bytes and passed its targeted test again. Listener controls
fail both SMTP and POP3 instances (two failed tests per mutant); each
restored filter executes two tests. Persistence, DATA and supplemental
stdio filters execute one each. No zero-test pass or compiler failure is
counted as a control.

| Control | Named failure | Mutant / restored exit |
|---|---|---|
| `harvest-removed` | isolation owner must harvest its completed session at admission | 101 / 0 |
| `harvest-shutdown-only` | isolation owner must harvest its completed session at admission | 101 / 0 |
| `harvest-first-batch-only` | first offered peer must be admitted | 101 / 0 |
| `harvest-shared-credit` | unrelated owner supplied harvest credit | 101 / 0 |
| `backoff-stacked` | stop did not finish before retry release | 101 / 0 |
| `premature-data-ack` | an unfinished commit was acknowledged | 101 / 0 |
| `data-read-timeout-bypassed` | DATA helper timeout mechanism did not fire while the peer was held | 101 / 0 |
| `stdio-peer-closes` | read timeout | 101 / 0 |
| `stdio-request-event-lost` | peer lost request event | 101 / 0 |

For each row, `<control>.log`, `<control>-restored.log` and
`<control>-<source basename>.patch` under the evidence root preserve the
command output and exact diff. `controls-ledger.json` records commands,
expected assertion, before/mutant/restored hashes and exit codes. Every
restored hash was compared with the final source table above. The two
stdio controls change only their peer fixture/event and are supplemental.

Prior evidence is retained in `control-attempt-1/`, `controls-v1/` and
`controls-v2/`. An initial harvest control hit the correct zero-count
assertion before the runner's expected greeting label; the assertion got
an explicit harvest diagnostic. Preliminary channel-inference compile
errors, a shared-future repoll failure, two clippy return-binding warnings,
and an untyped lost-event mutant were corrected. They are not behavioral
red proof. Final controls were rerun after the return/cleanup and timer
changes, and all hashes restored. The workspace client's 300-second
response timeout during the final controls + workspace-gate batch is
recorded in `transport-recovery.json`; the durable completed subprocess
receipts/logs were inspected afterward and prove their successful exits.

## Coverage failure and actual repair

The first new exact run, `target/coverage/run.nAN0pY`, failed at
12729/12727 lines, 756/756 branches, 1409/1409 functions. Its only misses
were the separate cfg(not(test)) timer bindings, SMTP 401 and POP3 293.
`gate-coverage-attempt1.log`, its original per-run LCOV/summary and the
failed source-bound row in `gates.json` remain preserved.

The owner now constructs one ordinary ACCEPT_BACKOFF timer in the shared
serve code; the cfg(test) readiness wrapper receives that same timer.
Release behavior, shutdown select and budgets are unchanged. This removed
no measured obligation: the successful run has the exact same denominator,
12729 lines, 756 branches and 1409 functions, all covered. No exclusion,
harness move, ignored test, floor change, gate normalization or retry for
green was used. Both backoff controls were rerun on the corrected source.

Final exact evidence: `target/coverage/run.6XQbGW/lcov.info` and
`coverage-summary.json`. `inline-observer-coverage.json` preserves raw DA
and BRDA records for both `reaped_until` replacements; it does no counting.
The pinned gate alone decides the literal 100/100/100 result.

## Final gate receipts

These are implementer workspace executions. They are not the downstream
hermetic verify/ship seats and are not the historical host receipt.
`gates.json` retains every attempt, command, timestamp, raw exit, pass
predicate and all eight source hashes. The latest row for every gate was
checked against the final source table; all thirteen are clean.

| Gate | Actual command | Verdict |
|---|---|---|
| `fmt` | `cargo fmt --all -- --check` | pass |
| `clippy` | `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass |
| `test` | `cargo test --workspace --locked` | pass |
| `bundle` | `.forge/bin/brokkr compile --bundle bundles/self` | pass |
| `coverage` | `bash scripts/coverage-gate.sh` | pass |
| `coverage-selftest` | `bash scripts/coverage-gate.sh selftest` | pass |
| `design-selftest` | `bash bundles/self/scripts/design-check.sh selftest` | pass |
| `delivery-proof` | `bash bundles/self/scripts/delivery-proof-test.sh` | pass |
| `publish-proof` | `bash scripts/publish-guard-test.sh` | pass |
| `seal-proof` | `bash scripts/seal-branch-test.sh` | pass |
| `licenses` | `cargo deny check licenses` | pass |
| `wrapper-digests` | `sha256sum --check --strict bundles/self/scripts/shims.sha256` | pass |
| `native-tls` | `cargo tree --locked -i native-tls` | pass (Cargo reports absent package with exit 101) |

Pinned brokkr SHA-256:
`e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`.
Compile reports 76 valuations / 14 groups / 0 unruled, with no unsafe sweep
warning. Native-tls absence requires Cargo's specific package-not-found
error; any successful inverse tree or unrelated Cargo error refuses.
Full workspace execution includes 323 passing library tests, API 64,
MCP 43, stdio 31 and smtp_edges 58; all other ordinary suites also pass.
Existing CI/release-only ignored tests remain as recorded; no ignored test
was introduced.

## Remaining delivery and CI evidence

T13 is pending for the existing engine: actual hermetic verify; independent
DashScope Qwen Max correctness, Claude Opus security, official DeepSeek
Flash guarantees (never Pro), DashScope GLM 5.3 tests; then actual ship.
No review or security hold is bypassed and no fallback/model change is made.
This implementation receipt closes neither that pipeline nor the epic.

CI-only Helm chart contract/history/selftest, real Docker image persistence
and context smoke, network RustSec audit, real port-80 bridge and tag-only
release-rate-5k were not performed by this seat. The remaining accepted
runtime exposures and contracts are unchanged; this lane adds no new
persistence, availability or delivery promise.
