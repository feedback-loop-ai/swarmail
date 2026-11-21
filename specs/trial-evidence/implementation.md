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

## Review return 3 (decision 0102), fixes

The second panel returned on `1453f9f` with lows only, two of them
security-relevant. Each is fixed with a test that turns red when the fix is
reverted (each was checked by reverting it):

| Finding | Fix | Commit | Test |
|---|---|---|---|
| A numeric sentinel in another spelling was published (low, security): `150` passed as `1.5e2`, `1.50e2`, `15.0e1` | A literal that is one JSON number is also kept as an exact decimal value (sign, significant digits, power of ten; no floating point). A JSON number of equal value, or a decimal number of equal value in any string and any decoded layer, is refused as `residual_secret`. **The choice for strings:** the registered spelling is masked as text as before, and any other spelling of the value is refused. A literal that is not exactly one JSON number (`0150`, ` 7`) is matched by spelling only | `61f276eb` | `toolvalues::tests::a_numeric_sentinel_is_found_by_value_in_any_spelling` (eight spellings as a number, nested, in a nested JSON string and in text; percent-encoded; a sign before it; negative and zero sentinels; near misses kept). Red with value matching off, and again with only the text check off |
| A form-encoded space hid a sentinel (low, security): `?q=my+secret` was published for `my secret` | The layer search runs a second reading with `+` as a space after each percent pass; a hit in either reading is a hit, so the form spelling is refused as `residual_secret` | `be8a3a2f` | `toolvalues::tests::a_form_encoded_space_never_hides_a_sentinel` (a path, a query, a form body, a nested JSON body, `%2B` and `%252B`, a key; `%20` and the literal still masked; a benign `+` kept; a row) |
| Other encodings were undocumented as unsupported | The README and spec AC3 now name the closed set: raw, JSON-escaped, `%XX` with `+` as space, and nested JSON, plus a numeric sentinel's value. Base64, HTML entities and other transformations are not decoded, and operators choose sentinels accordingly | `be8a3a2f` | — (documentation) |
| The registry could shrink during a trial (low, security): the sentinel file was read only at seal, so rewriting it to `[]` removed a sentinel and the shrunk registry approved the value and the row | The file is read once when the trial's capture starts, before any agent. The sealed registry is that snapshot united with the file at seal (`toolvalues::sealed_registry`), so it only grows. A file unreadable or malformed at seal keeps the snapshot and prints `tool_sentinels_unreadable_at_seal` on stderr; one unreadable at start leaves the trial `registry_unavailable` as before | `b5e94304` | `toolvalues::tests::the_registry_only_grows_during_a_trial` (rewritten to `[]`, a late addition, malformed, removed, the union's cap), and end to end in `tests/tool_values.rs::values_are_redacted_bound_and_never_scored`: the probe, now the matrix's last trial, rewrites the file mid-trial to hold only a late sentinel and sends both. The removed and the late sentinel are both `[REDACTED]`, and the late one is in the canary search. Red with the snapshot dropped, and red with the file at seal ignored |

The late-sentinel debt in `tasks.md` is closed by the last row.

Not changed here, and why:

- The unreadable-at-seal record is a stderr diagnostic, not a field of
  `capture.json`. Adding one would change the private record's schema for
  a case that never shrinks the registry.
- Over-refusal is the price of failing closed. `%2B` is also read as a
  space, and a numeric sentinel refuses every equal number in text, such as
  a count that happens to equal it. Both withhold evidence and never publish
  a secret.
- The remaining debt in `tasks.md` (capture-off control, contaminated
  metadata, replay row by row, report-CLI swap cases) is unchanged.

Time added by this return: the three new unit tests under 0.01 s each. The
end-to-end proof adds no trial; `tests/tool_values.rs` is 13.6 s (was 14.1 s,
host noise).

Gates after this return (`--locked`, this seat, 2026-11-20, tree
`b5e94304`, load average 13–17 at start): root `cargo fmt --all -- --check`
0; evals `cargo fmt --all -- --check` 0; evals `cargo clippy --all-targets
--locked -- -D warnings` 0; evals `SWARMAIL_BIN=<artifact record of cargo
build --locked --bin swarmail> cargo test --locked` 0, 600 passed, 0 failed,
7 ignored, 1550 s wall. No test failed, so none was rerun. As in return 2,
the root-only gates and `brokkr compile` were not rerun: nothing outside
`evals/` and `specs/trial-evidence/` changed.

## Review return 4 (decision 0102), fixes

The third panel returned on `9a5f153` with lows only, and with a spec
defect. AC3 both required rejecting unsupported encodings and said a
sentinel spelled in base64, HTML entities or another case is published.
The operator ruled on 2026-11-21: redesign the value mode as deny by
default. Each fix has a test that turns red when it is reverted (each was
checked by reverting it):

| Finding | Fix | Commit | Test |
|---|---|---|---|
| Spec defect: the bearer, which no operator chooses, published base64, entity or case-changed (low, security; correctness, guarantees) | Deny by default (decision below). After masking, every published name, string and number spelling must be printable ASCII with no backslash, no complete `%XX`, no `&...;` entity and no run of 37+ base64/base64url/base32/hex alphabet characters, else `invalid_argument_encoding`. Literals are searched ignoring ASCII case (`residual_secret`). Sentinels must be at least 32 bytes. No new reason | `760b0720` | `toolvalues::tests::deny_by_default_refuses_every_unreadable_spelling` covers the bearer and a sentinel as base64, base64 in text, base64url, hex, named/decimal/hex entities, upper/lower/mixed case, text-level `\u`, and fullwidth, each in a path, a body and a key; each shape rule alone; the fixture's synthetic data published unchanged. Red with the shape gate off, and red with case folding off |
| Numeric sentinel missed with a leading dot (low, security; correctness, security) | `Decimal` reads `.5`, `5.` and `.5e1`; numbers in text are read from every start, overlapping; a number whose digits contain a numeric sentinel's is refused | `760b0720` | `a_numeric_sentinel_is_found_by_value_in_any_spelling` (`.5`, `-.5`, `5.e-1`, `7.1.5`, `.05e1`, digit strings). Red with the leading-dot reading off |
| Registry fell back to the snapshot when the file broke at seal (low, security; correctness) | A file unreadable or malformed at seal poisons the registry (`registry_unavailable`); the snapshot ∪ file union stays for the readable case | `b684ed2a` | `the_registry_only_grows_during_a_trial`, now with a late sentinel in each of four malformed files: the seal is `registry_unavailable` with no row approval. Red with the fallback restored |
| Journal free text copied verbatim (low, security; independent) | Non-owned tool names published as `sha256:<hex>`; the run id published only when plain, else `null` | `854365a2` | `binding_follows_the_structural_sequence_exactly`, `a_companion_is_published_whole_or_absent_with_its_reason`. Red with either reverted |
| The canary search looked for raw spellings only (spec defect, tests) | The search ignores ASCII case and also looks for each literal's base64 (padded and not), base64url, hex and entity spellings. A new `encoded` agent sends the bearer and sentinel base64, base64url and entity-encoded; `planted` adds an upper- and mixed-case bearer; the oracle's captured arguments are checked to pass the plain shape whole; the probe's companion still publishes | `f2f28b55` | `tests/tool_values.rs::values_are_redacted_bound_and_never_scored`. With the shape gate off it is red at the `encoded` index entry, and the widened canary search alone is red too (checked by moving it ahead of the other assertions) |
| AC3 and the README | AC3 rewritten as the operator ruled, with a dated note; the review-return-3 sentence that published unsupported spellings is gone. The README states the deny-by-default rule, the charset and run limit with their reasons, the 32-byte minimum, the journal rule, over-refusal as the accepted cost, and what no shape check can see | `c6d30039` | — |

**The decisions, and why.**

- **Charset: printable ASCII, 0x20 to 0x7E.** Every fixture identifier,
  inbox, subject, code, token, link and the oracle's real arguments fall
  in it; the end-to-end test checks the oracle's captured arguments.
  Non-ASCII would admit fullwidth and look-alike letters, and invisible
  separators, that spell a literal past a case-insensitive search. Control
  characters would admit terminal and table injection. Backslashes are
  refused outright, so no `\u` or `\x` escape survives decoding.
- **Run limit: 37.** The fixture's longest opaque datum is a Swarmail
  message id, a UUID of 36 characters from the base64url alphabet. Mailed
  tokens are 24, passwords 16, codes 6. Any base64, base64url, base32 or
  hex spelling of a literal of 32 bytes or more holds a run of at least 40
  (base64 4 * floor((32 - 2) / 3); hex 64; base32 40). 37 is the least
  limit that publishes the fixture's data, and every literal is at least
  32 bytes: the bearer is 32 alphanumerics, and the sentinel file now
  refuses anything shorter. A run limit tied to the shortest sentinel (the
  ruling's example, min(sentinel length, 16)) would have refused every
  mailed token and message id, and so every trial that touches a link.
- **Leading dot.** The simpler fail-closed option was to read the dot,
  not to refuse every near-equal token.
- **Journal free text.** Digests for non-owned tool names, which need no
  registry, and the shape check for the run id, as ruled.

**Not done, and why.**

- A literal split across fields or calls, or re-encoded into
  ordinary-looking text, cannot be detected by any shape check. That
  boundary is accepted 0075 §9 (hostile same-user activity), stated in AC3
  and the README.
- The other review items stay as they were: `write_page`'s in-place write
  and U+2028/U+2029 in table cells, the capture root created before
  brokkr admission, `trial --isolated --capture-tool-values`, and the
  eval_http socket through the production wrapper. So does the tracked
  debt in `tasks.md`: no capture-off control, replay not row by row, and
  report-CLI zero-call and swap cases. Decision bodies are untouched.
- The fixture data that publishes is the data checked. A future task whose
  arguments carry longer opaque runs, non-ASCII text or entities will be
  withheld, the accepted cost.

Time added by this return: the new unit test runs in under 0.01 s. The
`encoded` agent is one more stand-in trial: `tests/tool_values.rs` took
16.5 s in the gate run at load average 35–39, against 13.6 s before; the
end-to-end proof alone takes 14.1 s, against 11.9 s.

Gates after this return (`--locked`, this seat, 2026-11-21, tree
`c6d30039`): root `cargo fmt --all -- --check` 0; evals `cargo fmt --all --
--check` 0; evals `cargo clippy --all-targets --locked -- -D warnings` 0;
evals `SWARMAIL_BIN=<artifact record of cargo build --locked --bin
swarmail> cargo test --locked` 0, 601 passed, 0 failed, 7 ignored, 1592 s
wall. No test failed, so none was rerun. The root-only gates and `brokkr
compile` were not rerun: nothing outside `evals/` and
`specs/trial-evidence/` changed.

## Review return 5 (decision 0102), fixes

The fourth panel returned on `af98ac3`: no spec defect, the
deny-by-default mode judged sound, and the lows below. Each fix has a test
that turns red when it is reverted (each was checked by reverting it):

| Finding | Fix | Commit | Test |
|---|---|---|---|
| A numeric sentinel with a leading zero, `+`, spaces or a trailing dot matched by spelling only (low, security) | Every literal that reads as a decimal number after trimming surrounding whitespace, one leading `+` and leading zeros (a leading or trailing dot allowed) is matched by value and by its significant digits. AC3 and the README say "every literal that reads as a decimal number" | `7549ce0a` | `a_numeric_sentinel_is_found_by_value_in_any_spelling`: `0150`, ` 7`, `+42`, `5.`, and a 32-byte all-digit sentinel with a leading zero caught spelled without it, as a number and in text. Red with strict-JSON-only registration |
| Forwarded request bodies bounded at the 16 MiB reply bound; the doc's worst-case claim was false (low, security) | Request bodies are capped at 2 MiB (`REQUEST_BODY_LIMIT`), mirroring Swarmail's `MCP_BODY_LIMIT` in the root `src/api.rs`, not imported from it; replies stay at 16 MiB. The module doc and the README state the real bound: 32 x (2 + 16) MiB for the trial's length | `f986b38c`, `0901fe86` (the first left a run-time constant assertion clippy refuses; the second makes it a const assertion) | `capture::tests::the_forwarder_bounds_a_request_body_at_swarmails_own_limit`, on a real socket: a body exactly at the limit is forwarded (200, upstream hit once). A body one byte over, declaring four times the limit and never sent whole, is answered 413 with no upstream hit. Red at the old cap (the read times out, bounded at 10 s) |
| The fixture's own `/invite/<token>/event.ics` ran to 38 and withheld every invite trial (low, correctness) | A run counts only `A-Z a-z 0-9 + - _`; `/ ? & = # .` and every other character break it. Every base64, base64url, base32 and hex spelling of a sentinel still holds an unbroken run of 40: `=` is only trailing padding, base64url's `-` and `_` count, and standard base64 writes `/` only for a `?`, DEL or non-ASCII byte in a group's third place. So sentinels are now printable ASCII other than `?` | `e91adcea` | `every_long_encoding_of_a_sentinel_holds_one_unbroken_run` (48 seeded literals over every admitted byte plus extremes and the bearer, six prefixes and three suffixes, padded and unpadded base64 and base64url, base32, both cases of hex, bare and inside a path: 55 literals, 13 860 spellings, each refused; why `?` and non-ASCII are refused). `deny_by_default_refuses_every_unreadable_spelling` now redacts one well-formed argument per advertised fixture route, the invite ICS path and its full `ics_url` included. Red with `/` and `=` as run characters, red with `-` and `_` as separators, red with `?` admitted |
| "Only the Only the" (README) | Fixed | `e91adcea` | — |

**Not done, and why.**

- An invite trial end to end is not run. No scripted agent reads the
  invitation and downloads the calendar, and adding one is a new agent
  and a new trial. The route is proven at unit level with its real shape.
- The other review notes stay as they were: `check --matrix` replay is
  compared by exit status and line count only, and there is no capture-off
  control run. Non-owned tool names are hashed only in the value files;
  the default-pinned structural `tool-calls.json` keeps them as written,
  unchanged by this mode. Ordinary-looking transforms (ROT13, a reversed
  bearer, decimal code lists) are the 0075 §9 boundary AC3 names.

**A host-load note.** The first gate run failed one root-build test,
`tests/runner.rs::automatic_build_uses_a_cleared_named_preparation_map`
(`swarmail_unavailable`), and failed again when rerun alone. Both failures
fell in the minutes when the shared `/tmp` tmpfs briefly reported ENOSPC
(other sessions hold several GB there); this seat's own tools failed to
start for the same reason. The test builds Swarmail with cargo into a
`/tmp` target directory and touches no code this return changed. Once
space returned, it passed alone (35.5 s), and the full rerun below passed
whole. The test is unchanged.

Time added by this return: `every_long_encoding_of_a_sentinel_holds_one_unbroken_run`
about 1 s (debug build, loaded host); the request-body test under 0.1 s.
`tests/tool_values.rs` 16.8 s in the gate run (unchanged, no new trial).

Gates after this return (`--locked`, this seat, 2026-11-21, tree
`e91adcea`): root `cargo fmt --all -- --check` 0; evals `cargo fmt --all --
--check` 0; evals `cargo clippy --all-targets --locked -- -D warnings` 0;
evals `SWARMAIL_BIN=<artifact record of cargo build --locked --bin
swarmail> cargo test --locked` 0, 603 passed, 0 failed, 7 ignored, 1594 s
wall (the rerun; the first run is in the note above). The root-only gates
and `brokkr compile` were not rerun: nothing outside `evals/` and
`specs/trial-evidence/` changed.

## Review return 6 (decision 0102), fixes

The fifth panel returned on `b449d42` with a regression from `e91adcea`.
With `=` and `/` breaking an opaque run, a quoted-printable spelling of a
32-byte literal (`=73=65...`) and hex pairs joined by `/` were published,
as `:`- and space-joined hex already were after return 4. It also flagged
a spec defect: AC3's "never in any spelling" claimed more than any shape
check delivers. The operator ruled on 2026-11-21 (review 5). Each fix has
a test that turns red when it is reverted (each was checked by reverting
it):

| Finding | Fix | Commit | Test |
|---|---|---|---|
| Quoted-printable of a literal published (low, security) | `=` counts in an opaque run again; two quoted-printable escapes in a row are refused; quoted-printable (`=XX` and soft line breaks) is a decoded layer of the residual search | `2f8ad27e` | `toolvalues::tests::quoted_printable_and_separator_joined_hex_are_refused`: the bearer and a 32-byte sentinel, quoted-printable in either case and with soft breaks, as a value, a key and in a path. Each rule has a case only it catches: a 37-run joined by one `=`, `x=41=42y`, and a sparse escape inside a sentinel with spaces. Red with each of the three reverted alone |
| Hex joined by any separator published (low, security) | Within a string, 64 or more hex digits joined only by characters that are not letters or digits are refused (`HEX_RUN_LIMIT`); a non-hex letter breaks the count | `2f8ad27e` | the same test: hex of both literals joined by `/ : space , newline - . = ", "`, in both cases, refused; 64 joined digits refused, 63 kept, a non-hex letter between two runs kept. Red with the rule off |
| Near misses must publish | — | `2f8ad27e` | the same test: one UUID, `/search?page=10&start=20261026T153000Z`, `token=4B<22>` in a path and a form, prose of hex-letter words ("deadbeef cafe ..."), 63 joined hex digits; two UUIDs side by side are refused (the accepted cost). `deny_by_default_refuses_every_unreadable_spelling` still publishes every fixture route and the full `ics_url` |
| The encoding test | Extended with quoted-printable (both cases) and hex joined by `:`, `/`, space and `,` | `2f8ad27e` | `every_long_encoding_of_a_sentinel_holds_one_unbroken_run`: 55 literals, now 25 740 spellings, each refused |
| End to end | A `joined` agent sends only the bearer quoted-printable and the bearer and sentinel as joined hex. It is withheld as `invalid_argument_encoding`, and was published under the previous shape. The canary search adds each literal's quoted-printable spelling and its hex joined by `:`, space, `/`, `,` and a newline | `87badc47` | `tests/tool_values.rs::values_are_redacted_bound_and_never_scored`. With `toolvalues.rs` from before `2f8ad27e` it is red at the `joined` index entry, and the widened canary search alone is red too (it finds the bearer in `capture.json`) |
| AC3 and the README promised "never ... in any spelling" (spec defect) | Both now state the exact guarantee: the bearer and every sentinel are refused in raw, ASCII-case-folded, JSON-escaped, `%XX`/`+`, nested-JSON, numeric-value, base64, base64url, base32, hex (any separators) and quoted-printable spellings. Both name what no check sees as the accepted 0075 §9 boundary: ROT13 and other letter substitutions, reversal, ascii85 and other unlisted alphabets, decimal or other code lists, literals split across fields or calls. The dated note "operator ruling 2026-11-21 (review 5)" is in both | `fae84cb0` | — |

**One deviation from the ruling, and why.** The ruling said to refuse any
complete `=XX` hex escape. A lone `=XX` is the shape of every `key=value`
whose value starts with two hex digits: `page=10`, `start=2026...`, and
about one random 24-character token in eight (`token=4B...`). The ruling
itself requires `token=<24>` to publish. So a lone `=XX` is kept, and
quoted-printable is caught three other ways, any one of which suffices:

- two escapes in a row are refused;
- the `=`-counting run catches a fully escaped literal (96 characters);
- quoted-printable decoding in the residual search finds any spelling of
  a literal, sparse escapes and soft line breaks included.

If the operator wants every lone `=XX` refused anyway, it is a one-line
change. It would withhold those form and token values.

**Not done.** As ruled, no check sees the alphabets and transforms named
in AC3's boundary, including per-byte code lists with a letter prefix
(`0x73,0x65`): the `x` breaks the hex count. The digit-string asymmetry
(the review's info 2: a numeric sentinel with a leading zero, written
inside a string between other digits) was left unchanged here, and this
ledger wrongly said the docs did not claim it: AC3 and the README do
claim digit-string matching for every literal that reads as a decimal
number. Review return 7 closes it (`935a61c4`). The earlier tracked debt
in `tasks.md` is unchanged.

Time added by this return: the new unit test runs in under 0.1 s, and the
extended encoding test in about 1.5 s with the lib harness. The `joined`
agent is one more stand-in trial: `tests/tool_values.rs` took 20.0 s in
the gate run, against 16.8 s before.

Gates after this return (`--locked`, this seat, 2026-11-21, tree
`fae84cb0`, load average 7–13): root `cargo fmt --all -- --check` 0;
evals `cargo fmt --all -- --check` 0; evals `cargo clippy --all-targets
--locked -- -D warnings` 0; evals `SWARMAIL_BIN=<artifact record of cargo
build --locked --bin swarmail> cargo test --locked` 0, 604 passed, 0
failed, 7 ignored, 1605 s wall. No test failed, so none was rerun. The
root-only gates and `brokkr compile` were not rerun: nothing outside
`evals/` and `specs/trial-evidence/` changed.

## Review return 7 (decision 0102), fixes

The sixth panel returned on `9fa09b6`: no spec defect, the lone-`=XX`
deviation accepted, one low security residual and a few lows. Each fix
has a test that turns red when it is reverted (each was checked by
reverting it):

| Finding | Fix | Commit | Test |
|---|---|---|---|
| A numeric sentinel's digits leaked inside text (low, security): `"id 3<31 digits>9"` for a `0` + 31-digit sentinel | `number_in_text` checks every number it reads by digit string as well as by value, with and without a `-` before it | `935a61c4` | `a_numeric_sentinel_is_found_by_value_in_any_spelling`: the sentinel's digits embedded with a prefix, a suffix, both, signed with a fraction, and inside an exponent spelling. Red with the digit check removed from the scanner |
| The capture root was made before brokkr admission, by a bare `DirBuilder` (low) | Admission (`admit_capture`) touches nothing and requires an existing parent that `artifacts::safe_root` admits; `create_capture_root` makes the 0700 root only after every other admission and checks it with the same rule | `820cee3c` | `capture_admission_tests::value_capture_admits_only_a_swarmail_target_and_a_creatable_root` (missing and symlinked parents, admission creates nothing, no second creation). `tests/tool_values.rs::matrix_capture_refuses_before_any_trial`: a matrix refused by brokkr admission leaves no root, and its retry with the same path runs (one oracle trial, 0700 root, one capture leaf). Red with the root created during admission |
| `--page` written in place (low) | Published through `artifacts::publish`: staged whole in a 0600 temporary beside it, moved with no clobber | `820cee3c` | `a_page_cut_short_is_removed`: after a write error nothing but the input is left. A writer killed by SIGXFSZ mid-write leaves no page at the name, and the published page is 0600. Red with the old in-place writer (the killed write left a partial page) |
| A 256-entry sentinel file poisoned every trial (info) | The file holds at most 255 sentinels (`SENTINEL_LIMIT`); the bearer takes the 256th slot | `691af87e` | `the_registry_is_bounded_and_fails_closed`: 255 read and sealed complete with exactly 256 literals, 256 refused. Red at the old cap |
| `EVAL_TOOL_CAPTURE_SOCKET` named nowhere (low) | Named in `evals/README.md`'s agent input table and socket section: harness-generated, a path and never a secret, only in capture mode, a ninth agent-visible input awaiting the operator's dated 0075 §1 addendum. `docs/evals/README.md` lists no agent inputs, so it is unchanged | `cb4f2d8e` | — |
| U+2028/U+2029 in page cells (low) | `escape_cell` turns them into U+FFFD | `cb4f2d8e` | `cells_are_escaped_and_figures_never_dash`. Red without the change |

Also corrected: the return-6 ledger paragraph said the docs did not
claim digit-string matching inside text. They do; that paragraph now
says so and points here.

**Not done.** Decision bodies are untouched. The ninth input and the
stale "not built" acceptance notes in 0075 and 0119 await the operator's
dated addenda. The tracked proof debt in `tasks.md` is unchanged.

Time added by this return: the retried matrix in
`matrix_capture_refuses_before_any_trial` is one oracle trial (about 2 s);
the killed-page case is under 0.1 s. `tests/tool_values.rs` took 22.3 s
in the gate run at load average 29–40, against 20.0 s before.

Gates after this return (`--locked`, this seat, 2026-11-21, tree
`cb4f2d8e`): root `cargo fmt --all -- --check` 0; evals `cargo fmt --all --
--check` 0; evals `cargo clippy --all-targets --locked -- -D warnings` 0;
evals `SWARMAIL_BIN=<artifact record of cargo build --locked --bin
swarmail> cargo test --locked` 0, 604 passed, 0 failed, 7 ignored, 1593 s
wall. The count is unchanged: this return extended existing tests and
added none. No test failed, so none was rerun. The root-only gates and
`brokkr compile` were not rerun: nothing outside `evals/` and
`specs/trial-evidence/` changed.

## Review return 8 (decision 0102), fixes

The seventh panel returned on `0616dac`: no security residual and no spec
defect, but one medium and some lows. Each fix has a test that turns red
when it is reverted (each was checked by reverting it):

| Finding | Fix | Commit | Test |
|---|---|---|---|
| A unit test would fail on the required macos-latest check (medium): `capture_parent` and `write_page` judged the parent as given, so macOS's `/var` -> `/private/var` was refused | Both parents are resolved with `artifacts::resolve_root` before `safe_root`, as `--artifacts-dir` and `--evidence-dir` are, so a root-owned symlinked ancestor is followed and any other is refused. A capture root that cannot be finished is removed again | `875ac2e2` | `capture_admission_tests::a_resolved_symlinked_ancestor_is_admitted_for_the_capture_root_and_the_page`. It follows a link under a temporary directory with a resolver standing in for the root-owned `/var` (the production rule rightly refuses a same-uid link); the capture root and the page land in the target. Red with the resolve removed from either function. The grep of this branch's tests found no other raw temporary path reaching `safe_root` unresolved: the rest resolve first or bind sockets only |
| A matrix refused after its root was made left the root (low) | When the matrix ends refused (exit 2), the root it created is removed if still empty; a root holding a record is kept | `1e1d5a1d` | `tests/tool_values.rs::matrix_capture_refuses_before_any_trial`: `--trials 0` and an unparsable agents file each leave no root, and the retry with the same path runs. Red without the removal. A half-made root is the first row's fix |
| `read_sentinels` refused a bare relative name (low) | An empty parent is the working directory | `622dbb72` | `tests/tool_values.rs::a_trial_reads_its_capture_paths_relative_to_its_working_directory`: one oracle trial with `captures` and `sentinels.json` as bare names seals complete with two literals. Red as `registry_unavailable` without the fix |
| The page's cached-tokens cell (low) | The cached sum is shown only when the group's token facts were joined whole (a figure, or `no_passes` with a journal); otherwise `null` with the same reason as the figure beside it | `02179b00` | `cells_are_escaped_and_figures_never_dash` and the CLI page test, whose `\| 0 \|` partial sum is now `null (incomplete_join)`. Red with the old bare `null` |
| A short-significand numeric sentinel refused nearly everything (info) | The option taken is a refusal at read time, not a README warning: a sentinel that reads as a decimal number needs at least 16 significant digits (`NUMERIC_SENTINEL_DIGITS`) | `dab5ea50` | `the_registry_is_bounded_and_fails_closed`: thirty-one zeros and a one, and 15 digits padded with spaces, refused; 16 digits zero- or space-padded admitted. Red without the check |

Left as they were, as asked: the CI time of `tests/tool_values.rs` and the
tracked proof debt in `tasks.md`. The review's other lows (no pre-change
matrix-row golden; no multibyte argument end to end, which production now
withholds) are recorded here and not addressed in this return.

Time added by this return: one more oracle trial
(`a_trial_reads_its_capture_paths_relative_to_its_working_directory`,
about 2 s); the two new refusal runs are under 0.2 s. `tests/tool_values.rs`
took 25.0 s in the gate run at load average 21–24, against 22.3 s before.

Gates after this return (`--locked`, this seat, 2026-11-21, tree
`dab5ea50`): root `cargo fmt --all -- --check` 0; evals `cargo fmt --all --
--check` 0; evals `cargo clippy --all-targets --locked -- -D warnings` 0;
evals `SWARMAIL_BIN=<artifact record of cargo build --locked --bin
swarmail> cargo test --locked` 0, 606 passed, 0 failed, 7 ignored, 1623 s
wall. No test failed, so none was rerun. The macOS suite is CI's; this
seat runs Linux only. The root-only gates and `brokkr compile` were not
rerun: nothing outside `evals/` and `specs/trial-evidence/` changed.

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
