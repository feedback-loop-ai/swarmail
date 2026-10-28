# Implementation record — MCP refusal proof

Implements [spec.md](spec.md) per [plan.md](plan.md). Test-and-evidence only:
no production, dependency, bundle, gate, fixture or decision change. The diff
touches `src/deliverability/concurrency_tests.rs` (the declared cfg(test) owner
harness, decision 0020 rule 2), `tests/mcp_revisions/structured.rs` and this
supplement. Historical specs, captures, decisions and supplied evidence are
untouched. This record is not engine verification or a reviewer verdict.

## Provenance (T1)

Starting HEAD `7bdb1e1` (chief spec commit) over candidate `8d205d7`, clean.
Passed: `.forge/epic192/issue153/SHA256SUMS` (directory-relative), the
root-relative manifests of `.forge/evidence/prior-153`, `unavailable-wire-gates`
and `issue152-annotation-review`, `.forge/bin/brokkr` against
`brokkr.binary_sha256` (`ab935558…885f`) and `bundles/self/scripts/shims.sha256`.
Baseline `cargo test --locked --lib report_workers_do_not_hold_store_guards`
exited 0 before any edit. Run-local logs live under the ignored
`.forge/evidence/refusal-proof-impl/`.

## Witnesses added or strengthened

| Witness | Evidence class | What it pins |
| --- | --- | --- |
| `deliverability::concurrency_tests::report_workers_do_not_hold_store_guards` | March HTTP; June/November HTTP and bridge-function | Complete literal unavailable envelopes (March id 1, modern id 2) instead of selected fields; bridge reply compared to the literal, not to the HTTP reply; negotiated revision, two-line cardinality, exit 0; no worker entered and zero permits while refusing. REST 503, SMTP 250 + queryability, oldest-only prune and post-hold delete/clear retained |
| `deliverability::concurrency_tests::report_unavailable_notifications_keep_bridge_silent` | June/November HTTP and bridge-function, all JOBS workers held | Unavailable report notification: HTTP 202 with zero body bytes, then correlated ping; bridge input init/initialized/notification/ping yields exactly init + ping after EOF drain, exit 0; no worker entered, zero permits, mail still stored; release and join every held worker |
| `structured::modern_report_argument_refusals` | June/November HTTP and spawned binary | Omitted arguments, `{}`, `id:null`, numeric id, null/array/string/number container, extra key, and JSON-RPC `id:null` request control: complete `-32602` envelopes with the sent id (`missing id`, `id must be a string`, `arguments must be an object of named arguments`, `unknown argument: extra`); a delivered mail's report still succeeds afterwards; every child finished |
| `structured::modern_notification_refusal_contract` | June/November HTTP and spawned binary, fresh server each | Seed notification plus three malformed report notifications: HTTP `(202, "")` raw, binary no line after EOF drain with clean exit/stderr; correlated ping; seed stored exactly once, nothing in `default` |
| `structured::modern_storage_errors_and_revision_isolation` | June/November HTTP and binary | Null and 60001 wait timeouts now compared to complete correlated `isError:true` envelopes |

## T8 ledger — final disposition

| Obligation | Witness | Status |
| --- | --- | --- |
| Default/explicit March bytes | `structured::legacy_bytes`, `historical_twelve_tool_fixture_is_preserved` | Retained, passing |
| Missing/numeric id, extra key | `structured::modern_all_tools_http_and_stdio` + `modern_report_argument_refusals` | Proved, modern HTTP/binary |
| Null report id; omitted/null/array/scalar arguments | `structured::modern_report_argument_refusals` | Proved, modern HTTP/binary |
| Missing mail | `structured::report_stored_bytes_over_modern_wire` | Retained, passing |
| Failed analysis | `structured::modern_all_tools_http_and_stdio` workflow (directly inserted historical row) | Retained; restored-row attribution, not SMTP or unavailable |
| Unavailable analysis | `report_workers_do_not_hold_store_guards` | Proved over modern HTTP and bridge-function. **Binary (spawned executable) unavailable: unproved** — the Held seam is cfg(test) in the library; the integration binary has no seam and none was invented |
| Storage refusal | `structured::modern_storage_errors_and_revision_isolation` | Retained, passing |
| Invalid wait timeout | same | Strengthened to complete envelopes |
| Shutdown | `structured::modern_parked_wait_shutdown_keeps_error_shape`; `report_wait_shutdown_interrupts_worker_and_slot_waits` | Retained, passing |
| Rate | `mcp::tests::exhausted_tool_rate_refuses_before_effects_and_keeps_control_available` | Direct dispatch only. **Shared production-router wire exhaustion: unproved (inherited LOW)** |
| Mixed revisions | `structured::modern_storage_errors_and_revision_isolation` | Retained, passing |
| Notifications | `modern_notification_refusal_contract`; `report_unavailable_notifications_keep_bridge_silent` | Proved: seed/malformed on modern HTTP/binary; unavailable on modern HTTP/bridge-function only. Binary-unavailable and wire-rate notifications unclaimed |
| Envelope/bridge bounds | revision/lifecycle suites, `structured::real_bridge_dual_output_size_cost` | Retained at the unchanged 10 MiB cap |

T8 of [mcp-report-output](../mcp-report-output/tasks.md) is not discharged by
this supplement: the binary-unavailable and shared-router-rate rows stay open.

Focused exits on the implementation tree (all 0): `--test mcp_revisions`
(25 passed), `--test stdio` (39), `--test deliverability` (7),
`--lib deliverability::concurrency_tests` (9) and the direct-dispatch rate test.

## Local gates (T7)

The whole house inventory plus the framed proofs passed on the clean unsigned
implementation commit `4227aef` (tree `67fe482`): fmt, locked workspace clippy
with warnings denied, locked workspace tests, digest-matched
`.forge/bin/brokkr compile --bundle bundles/self`, exact coverage
(17372/17372 lines, 1202/1202 branches, 1781/1781 functions) and its selftest,
publish-guard and keyless seal proofs, `cargo deny check licenses`,
`design-check.sh selftest`, `delivery-proof-test.sh`, the strict shims check,
fail-closed native-tls absence (`cargo tree --locked -i native-tls`, exit 101
with the no-package diagnostic) and `design-check.sh check .`. The same
inventory is rerun on the commit that records this section; receipts with
command, exit, HEAD/tree and log hashes are run-local under
`.forge/evidence/refusal-proof-impl/`. CI-owned contexts (macOS, Helm, Docker,
RustSec, port-80 bridge) are not represented here and remain owed.

## Handoff (T8 of this supplement stays open)

Engine verification and the five separate protected reviewers own their
verdicts; nothing here substitutes for them. Inherited findings keep their
original member-specific severities: run `20b54c48` (event
`ac3d0439-c41c-4415-8296-544de921621c`, seq 834) tests indeterminate, and the
settled correctness, guarantees, independent and security members' HIGH
security-relevant inherited review-integrity/result-attribution residual;
MEDIUM actual-client gaps; LOW OPTIONS, shared-rate, documentation and route
findings; INFO as recorded. Parent run `5572d327` review remains live. Still
owed before landing: the parallel #152 OPTIONS candidate's actual review and
combined-candidate proofs, #151 paid baseline (operator), #201 host-wide eval
coordination, actual-client receipts, CI-owned checks and the operator seal.
No push, merge, tag, issue closure, paid trial or decision acceptance occurred.
