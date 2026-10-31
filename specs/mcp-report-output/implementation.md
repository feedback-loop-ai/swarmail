# Implementation record — structured MCP report output

Implements [spec.md](spec.md) under proposed
[0105](../../docs/decisions/0105-mcp-report-output.md) on design commit 848aa65.
Witness names below are the tests that prove each criterion on this candidate;
gate receipts belong to the implement seat's result and engine verify.

## Production change

`src/mcp.rs` only. `OUTPUT_SCHEMAS` gains the closed report schema (finding,
three tagged locations, severity enum, u64 coordinates, `maxItems` from
`deliverability::MAX_FINDINGS`, the two OpenAPI patterns). Modern descriptors
replace the stale report description; the shared `TOOLS` literal, and with it
March, is unchanged. The formatter, error helper and report selection are
untouched. No dependency, module, knob or bound changed.

## Evidence by criterion

| AC | Witness | Class |
| --- | --- | --- |
| AC1 | `revision_tool_annotations` (exact 13 names in order, annotation table, object schemas, March without schema/annotations); `mcp::tests::every_tool_has_exactly_one_object_output_schema` (schema key set = catalogue) | real HTTP; inline |
| AC2 | `report_schema_typed_forms_and_negatives` (every location form, null occurrence, empty paths/report, 0 and u64::MAX, 256 accepted, 257 and ~40 shape/tag/bound/pattern negatives) | typed-only (Link, empty report and extreme coordinates are not production-reachable) |
| AC2/AC5 | `report_stored_bytes_over_modern_wire` (real SMTP bytes; demo info/warning/error, header occurrence 0 and null, root MIME path; complete stated findings = REST = structuredContent = text, June/November x HTTP/stdio, schema-valid; not-found; March text-only) | real wire |
| AC3 | `evaluator_selftest` (enum, minItems/maxItems, character minLength, both patterns cross-checked with the regex crate, exact bounds at 2^53, i64 ends, u64::MAX, 2^64, integral floats, fractional comparisons; malformed/unsupported keywords and patterns refused, including in unused arms; ambiguous oneOf) | test-only evaluator |
| AC4 | `served_openapi_pins_and_negative_controls` (six existing + four report pins in `openapi-report-source.json`; source drift for each report constraint; complete projections equal served MCP schemas on both modern revisions; projection mutations detected) | served OpenAPI |
| AC5 | `modern_all_tools_http_and_stdio` (all 13 successes per revision/transport; request log = baseline case identities; equal lengths; each success line = March line + identical structuredContent bytes; errors byte-equal) | real wire vs independent baseline |
| AC6 | `legacy_bytes` (missing/explicit March over HTTP and the real bridge = `march-current-main.jsonl`, requests = recorded log); `historical_twelve_tool_fixture_is_preserved` (`march.jsonl` rows equal the shared baseline rows; catalogue differs only by the report tool and the wait threshold) | independent capture |
| AC7 | workflow report refusals (not found, missing id, non-string id, unknown argument, analysis failed on restored bytes); existing storage/rate/shutdown/notification and `real_bridge_dual_output_size_cost` witnesses unchanged | real wire |
| AC8 | unchanged suites: mcp, p2, stdio, deliverability, store and owner tests | existing |
| AC9 | README / llms.txt report paragraphs; `report_served_docs_match_contract` expectation retargeted from the temporary "when #153 lands" clause; 0105 indexed as proposed | docs |

## Baseline capture

`tests/fixtures/mcp-structured-output/march-current-main.provenance.json`
records source 3a2eef9 (tree ce2e41e, re-hashed blob-for-blob), the test-only
capture harness and case list, build invocation, executable digests before and
after, and the single `$SEED_ID` substitution. 3a2eef9 is a local candidate, not
a protected #152 acceptance receipt.

2026-10-31 (#259, PR #292): the seed report row moved from `swarmail-bulk/1` to
`swarmail-html/3` by a recorded one-token edit, not a re-capture; the provenance
`updated` field and proposed [0107](../../docs/decisions/0107-html-review-repair.md)
say why.

## Gaps stated, not waived

- The MCP `deliverability analysis unavailable` text is not reached on the wire:
  it needs cache or job exhaustion. It shares the adapter line with the failed
  case exercised here; the owner test in `src/deliverability/concurrency_tests.rs`
  proves the condition itself.
- Link locations, empty reports and extreme coordinates have typed evidence only.
- Actual #152 baseline and its protected review, paid #151 baseline, real client
  matrix, CI, operator seal and the five engine review results remain outstanding.
- Standalone eval suites were not run: no authentic host-wide same-user
  nonoverlap is available from this seat.
- Historical specs, 0082, `march.jsonl`, `openapi-source.json` and
  prior-runs.json are unchanged.
