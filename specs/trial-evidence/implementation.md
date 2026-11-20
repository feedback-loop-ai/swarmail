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

## Review return 1 (decision 0102), fixes

Every finding above low and every security finding, by member:

- **Percent-decoding fail-open (medium, security; correctness, guarantees,
  security, independent).** Fixed in `toolvalues.rs`: each complete `%XX`
  escape is decoded wherever it stands, JSON and percent layers alternate
  over bytes to a fixed point (bounded at 32 rounds; unsettled text counts
  as holding a literal), the final residual check uses the same layers, and
  a percent decoding that never settles or is not UTF-8 is refused
  (`invalid_argument_encoding`). The reviewer's probes (`%53…%zz`, `%2573…`)
  and mixed/non-UTF-8/unsettled cases are unit tests; the probe agent now
  sends a malformed-escape and a double-encoded spelling end to end, and
  the canary search also looks for each literal's tail.
- **Capture socket reachable by the agent (medium, guarantees; low,
  security: unbounded `open` and accept loop).** Answered and hardened. The
  path must reach the helper through the agent's own MCP configuration, so
  no CLI-launched helper can hold a secret the agent cannot read; the spec's
  "no interface to declare trusted tools" holds because a frame can only
  report `eval_http` (`Server::Fixture` admits that one name; the catalogue
  is never reachable through the socket), and every published value must
  match the journal's digest for the agent's real call, so a forged frame
  only withholds the trial. Now bounded: one open call per connection, at
  most 16 connections per trial (`HELPER_LIMIT`); either excess latches
  `binding_mismatch`. Stated in `capture.rs` and `evals/README.md`; unit
  tests in `capture::tests`.
- **Mask collision in values (low, security).** Fixed: a string that already
  spells `[REDACTED]`, directly, JSON-escaped, nested or in its percent
  decoding, is refused (`invalid_argument_encoding`).
- **`respell_origin` never executed (medium, tests).** Unit cases in
  `capture::tests`, and on the wire: the probe sends the forwarder's own
  origin (Swarmail answers only once it is respelled; mutating the
  respelling to a pass-through turns the test red with `403 Forbidden`,
  checked) and a foreign origin, which Swarmail refuses.

Lows also closed: multi-call batches are documented and tested as
`binding_mismatch`; the lost observer and the `unreadable` frame are tested
(`fixture_http::tests`); a `sum_overflow` page cell is tested; a page write
that fails part-way removes its own file; the decision index no longer calls
the mode unbuilt. Lows left as tracked debt are named in `tasks.md` (items
2, 3, 4, 7, 8), plus the observer's bounded 2 s acknowledgement wait (now
stated in `capture.rs`), the four untested fail-closed diagnostics
(`capture_requires_swarmail_target`, `capture_root_unavailable`,
`tool_capture_write_failed`, `eval_capture_socket_invalid`), the coarse
`capture_missing` for a private write failure, and the CI-time overrun below.

Time added by this return: `tests/tool_values.rs` 10.1–10.4 s (unchanged:
no new trial), new unit tests under 0.01 s each.

Gates after this return (`--locked`, this seat, 2026-11-20): root fmt,
clippy, test (37 suites ok), `cargo build --locked --bin swarmail`; evals
fmt, clippy and `SWARMAIL_BIN=<artifact record> cargo test --locked` (590
passed, 0 failed, 23 min); coverage gate exact 100/100/100 and its selftest;
publish-guard and seal-branch proofs; `cargo deny check licenses`;
native-tls absent in both trees: all 0. `brokkr compile` over every recipe:
not run, no pinned brokkr is reachable from this seat; `.brokkr/` is
unchanged.

## Review return 2 (decision 0102), fixes

Five members (correctness, guarantees, independent, security, tests)
returned on `6e6be3a`. Each finding below is fixed with a test that turns
red when the fix is reverted (each was checked by reverting it):

| Finding | Fix | Commit | Test |
|---|---|---|---|
| Decoded mask collision (low, security; correctness, guarantees, security): `%5BREDACTED%5D`, `x %5BREDACTED%5D y`, `%255BREDACTED%255D`, a nested `{"b":"%5BREDACTED%5D"}` passed unchanged unless a literal was also present | Every string and name is searched for the mask through the JSON-escape, percent and nested-JSON layers before masking; refused `invalid_argument_encoding`, literal or not | `18b4a74b` | `toolvalues::tests::a_decoded_mask_is_refused_without_a_literal` (each spelling as a value, a name and a nested element) |
| Row approved by a raw byte search (low, security; all three security-relevant members): an escaped or percent-encoded sentinel reached a transcript | `Registry::found_in_document`: the parsed row's names, strings and numbers through the same layers; an unparsable row is not approved | `4bf7471a` | `toolvalues::tests::a_row_holding_an_escaped_or_encoded_literal_never_reaches_a_transcript` (a sentinel with a quote, a backslash and a control character, and a percent-encoded one: both rows skipped `row_unverified`, no leaf) |
| Forwarder without connection or request cap (low, security) | At most 32 connections open at once (`CONNECTION_LIMIT`; one more is closed unserved), at most 512 requests observed per trial (`REQUEST_LIMIT`; later ones forwarded unread, so the trial runs on). Either latches the existing `trial_too_large`; no new reason. A capture whose socket bind fails now stops its forwarder (review info 6d) | `c7c0aaff` | `capture::tests::the_forwarder_serves_a_bounded_number_of_connections`, `::the_forwarder_observes_a_bounded_number_of_requests` (real loopback sockets, at the cap kept, one over refused, ordered by replies and closes, no sleep) |
| Built-in group printed `null (incomplete_join)` with a journal (low) | `tokens_figure` reads the built-in surface before the journal | `ed7c605a` | `toolvalues::tests::cells_are_escaped_and_figures_never_dash`, `tests/tool_values.rs::the_page_gives_every_paid_group_a_figure_or_its_reason` (journal-present oracle cell) |
| Unexecuted diagnostics and page removal; no published zero-call envelope (low, tests) | Tests only | `ece77539` | `capture_admission_tests::value_capture_admits_only_a_swarmail_target_and_a_creatable_root` (`capture_requires_swarmail_target`, `capture_root_unavailable`); `tests/tool_values.rs::matrix_capture_refuses_before_any_trial` (`capture_root_unavailable` on the CLI); `::a_capture_that_cannot_be_written_leaves_the_row_and_says_so` (`tool_capture_write_failed`, the oracle row still `pass`); `tests/wrappers.rs::exact_eight_input_admission_and_literal_values` (`eval_capture_socket_invalid`); `::a_page_cut_short_is_removed` (a 64-byte file-size limit cuts the page; the file is gone); `toolvalues::tests::a_companion_is_published_whole_or_absent_with_its_reason` (zero-call envelope through `publish_companion`) |
| Operator runbook built the wrong workspace (info) | `--manifest-path evals/Cargo.toml` | `92548131` | — |
| Ledger counts 590 vs 586 (info) | Corrected in the gate table above | this commit | — |

Not changed here, and why:

- The 0075 and 0119 decision bodies still say the value mode is unbuilt
  (guarantees info 4). Only the operator annotates a decision; the index
  rows already say built.
- Still tracked as debt in `tasks.md`: no capture-off control compared call
  by call, no separate late-sentinel case, no contaminated-metadata case,
  `check --matrix` replay compared by exit status and line count, no
  report-CLI zero-call or swapped-capture case, `read_capture`'s deep
  validation arms, the `transcripts_write_failed` CLI arm, and
  `write_value_capture`'s capture-never-started arm. Also the info items
  `bind()` re-parsing the checked value, the whole-string percent decoding
  on a literal hit, and `trial --isolated --capture-tool-values` ending in
  `capture_missing`.
- The real-engine journal digest join stays unobserved until the operator's
  smoke.

Time added by this return, measured inside the test binaries at load
average 17–42: the two forwarder tests 0.3 s together, the new unit tests
under 0.1 s, `a_page_cut_short_is_removed` 0.2 s, and
`a_capture_that_cannot_be_written_leaves_the_row_and_says_so` 2.2 s (one
oracle trial). About 3 s in all. `tests/tool_values.rs` is now 14.1 s, of
which 10–12 s is the existing end-to-end proof.

Gates after this return (`--locked`, this seat, 2026-11-20, tree
`92548131` plus this ledger): root `cargo fmt --all -- --check` 0; evals
`cargo fmt --all -- --check` 0; evals `cargo clippy --all-targets --locked
-- -D warnings` 0; evals `SWARMAIL_BIN=<artifact record of cargo build
--locked --bin swarmail> cargo test --locked` 0, 597 passed, 0 failed, 7
ignored, 1601 s wall. No test failed, so none was rerun. Root clippy, root
test, coverage gate and its selftest, publish-guard and seal-branch proofs,
`cargo deny` and `brokkr compile` were not rerun by this return. It touched
only `evals/` and `specs/trial-evidence/`, outside the root build and
coverage denominator. CI and the verify seat own them.

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
| `cd evals && SWARMAIL_BIN=<candidate> cargo test --locked` | 0 (590 passed, 23 min wall; review return 1). This row said 586, a stale count from before that return's last tests; 590 is the count the return's own gate paragraph recorded. Review return 2: 597 passed, 0 failed, 7 ignored, 1601 s wall (below) |
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
