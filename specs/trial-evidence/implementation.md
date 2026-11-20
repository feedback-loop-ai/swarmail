# Trial evidence — implementation record

Run `issue-391-record-tool-calls-toke-caff20b7`, built on `0d4839f` (the
design commit). Part of #391; this delivery does not close it.

## What landed

| Piece | Where | Proof |
|---|---|---|
| Default contract pinned before any edit | `evals/tests/data/default-contract/` (produced by the 68c3839-content binary) | `tests/report.rs::the_default_report_is_byte_identical_to_the_pre_change_contract` (stdout with and without a journal, the structural tree, absences included) |
| Strict span-keeping JSON reader, sealed registry, redaction, bounds, capture latch, binding, publication, transcript selection, page | `evals/src/toolvalues.rs` | `src/toolvalues/tests.rs` (12 tests, 0.3 s) |
| Trial-side capture: loopback forwarder in front of Swarmail's `/mcp`, acknowledged helper socket | `evals/src/capture.rs`, `runner.rs` (`CapturePlan`, seal after every agent is reaped, write after the row is final), `fixture_http.rs` (`Observer`), `wrappers.rs` (`capture_socket` passed to the helper only when set) | `tests/tool_values.rs::values_are_redacted_bound_and_never_scored` |
| Flags | `matrix --capture-tool-values ROOT --tool-sentinels FILE` (also on `trial`, rendered into isolated and brokkr requests); `report --tool-values-from ROOT` (requires `--journal`, `--evidence-dir`); `report --page PATH` | `tests/tool_values.rs::matrix_capture_refuses_before_any_trial`, `::the_page_gives_every_paid_group_a_figure_or_its_reason` |
| Journal join | `journal::write_values`; `journal.tool_values` per group only in the value mode | the end-to-end test compares the value report minus `tool_values` with the default report |
| Docs | `evals/README.md` ("Value-bearing evidence", the operator smoke), `docs/evals/README.md` status, `specs/eval-brokkr-recipe/tasks.md` T43–T48, `docs/decisions/README.md` row for proposed 0136 | — |

## Deviations from the plan, with reasons

1. **The registry is runner-owned, not a new fixture inventory.** The runner
   generates the fixture admin bearer; the only other literals are the
   operator's sentinels (`--tool-sentinels`, a JSON array, read through the
   bounded reader at trial end). The fixture issues no other private literal:
   its tokens, codes and links are the synthetic task data 0119 §4 says the
   values carry by construction, and masking them would erase the diagnosis
   the mode exists for. So no fixture route or endpoint was added. A missing,
   malformed or over-cap sentinel file at trial time is `registry_unavailable`.
2. **Overlap is refused, not disambiguated.** A call that starts while another
   harness-owned call is in flight withholds the trial (`binding_mismatch`).
   That includes a CLI's parallel tool calls and the two identities of an
   isolation task sharing one forwarder. Wrong attribution costs more than
   lost evidence.
3. **Copied identifiers are not re-checked against the registry at report
   time** (the registry is gone by then). The row is checked while the
   registry is live (`row_sha256` in the capture), and only an approved row
   is copied into a transcript. `run_id` and tool names come from the facts
   reader's finite, validated identifiers.
4. **Competitor targets refuse capture** (`capture_requires_swarmail_target`):
   their endpoint is the runner's bridge, which this mode does not observe.
5. **`trial --capture-tool-values` needs an existing root**. `matrix` creates
   a fresh one (0700) and refuses an existing path (`capture_root_exists`).
   A trial whose capture never started, or failed to write
   (`tool_capture_write_failed` on stderr), leaves the row untouched and the
   report says `capture_missing`.

## Held paths

None touched: `git diff --stat 0d4839f -- Cargo.toml Cargo.lock evals/Cargo.toml evals/Cargo.lock deny.toml scripts .github .brokkr src tests` is empty. No dependency or feature was added.

## Time added to CI (measured 2026-11-20, loaded host, load average 9–16)

- `tests/tool_values.rs`: 10.6 s (one stand-in matrix of four trials: oracle,
  null, the probe and the planted probe; six report runs). Above "a few
  seconds": the four trials are the ACs' controls and the planted refusal, and
  each brokkr-mode trial costs about 2.5 s here.
- `src/toolvalues/tests.rs`: 0.3 s.
- `tests/report.rs::the_default_report_is_byte_identical_to_the_pre_change_contract`: about 1 s with `SWARMAIL_BIN` set.
- Total about 12 s. The real-binary test reads `/proc`. It carries the
  non-Linux ignore.

## Gates (this seat, `--locked`)

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (2187 passed) |
| `cargo build --locked --bin swarmail` | 0 |
| `cd evals && cargo fmt --all -- --check` | 0 |
| `cd evals && cargo clippy --all-targets --locked -- -D warnings` | 0 |
| `cd evals && SWARMAIL_BIN=<candidate> cargo test --locked` | 0 (586 passed, 23 min wall) |
| `brokkr compile` over `.brokkr/recipes/*/` | **not run: no pinned brokkr binary is reachable from this seat.** `.brokkr/` is unchanged; the verify seat and CI's `brokkr bundle (pinned)` run it |
| `bash scripts/coverage-gate.sh` | 0, exact 100/100/100 on the second run. The first run failed one root test, `mail_compare::api_section::preparation_refuses_when_the_target_does_not_answer` ("smtp read: Connection reset by peer"), at load average 16. The test frees a port and expects a refused connect, so another process can take the port meanwhile. Root code is untouched here, and the test passed in the plain root run |
| `bash scripts/coverage-gate.sh selftest` | 0 |
| `bash scripts/publish-guard-test.sh` | 0 |
| `bash scripts/seal-branch-test.sh` | 0 |
| `cargo deny check licenses` | 0 |
| native-tls absent (`cargo tree -i native-tls`, root and evals) | no package matches |

CI alone proves the macOS suite (the end-to-end test is ignored there), the
Helm, Docker and RustSec jobs.

## Not done here (operator-owned)

The first authenticated via-brokkr smoke per family (`evals/README.md`),
its dated addenda in 0119 §§5–6, a ruling on proposed 0136, and any paid run.
`--via-brokkr` still refuses `driver_contract_unknown`.
