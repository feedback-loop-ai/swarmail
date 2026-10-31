# Implementation record — focused review repair

Implements [spec.md](spec.md) per [plan.md](plan.md) and proposed
[0097](../../docs/decisions/0097-focused-review-repair.md). This record is not
engine verification or a protected verdict; no earlier candidate's verdict or
verify transfers. Local gate receipts are bound to the final commit outside
the tree (a tracked receipt would change the tree it proves); the engine's
verify seat reruns every gate on the pinned bundle.

## Sources (T1)

Starting HEAD `2d16d9b` (chief spec commit, tree `4cb238e0…`) over the
commissioned candidate `5dc3bc94dfa56d9fcf80efc6587a6de22d6a84a1`, tree
`7da304fe0ff71283ac9f03f8b1a91956f066044e`, on provisional #153 `25b367d`
(tree `91c8ada3…`). Checked before editing: the panel event's SHA-256
`b1ebc081…891a`, `.forge/bin/brokkr` against `binary_sha256` (`ab935558…885f`)
and `bundles/self/scripts/shims.sha256`.

| Source | Disposition on this stack |
| --- | --- |
| #153 `393247c` (sibling of `25b367d`, differs only in one assertion) | Ported: every output schema root asserts `type: object` separately from the absent root `$ref`, over fifteen tools. |
| #153 `8d205d7`, `7bdb1e1`, `4227aef` | Cherry-picked unchanged: unavailable-report transport, exact modern refusal envelopes and notification silence; their spec records come with them. |
| #153 `df8f610` | Not imported: receipts bound to #153's own tree, not this one. |
| #152 `f0b2b14` (ancestor of `7f2e2129`) | Ported with adaptation: OPTIONS 405/Allow: POST/no CORS whatever the version header (accepted 0043 §4), router-owned shared tool limiter and its seam test, binary tools/list parity; catalogue checks name fifteen tools with nine reads; both focused waits join the shared-budget refusal set; #153's structured-output prose is kept over the parent's no-outputSchema sentence. Its proposed 0090 record and index row come with it. |
| #152 `91e6890` | Already present (report read-only hint). |
| #152 `9317cd1`, `70e85fc`, `7f2e2129` | Not imported: they link and specify proposed 0092 (mcp-candidate-proof), whose thirteen-tool qualification does not describe this fifteen-tool stack. Reconciling it is owed when #152's reviewed head exists. |
| #152 `ab80743` | Not imported: a model-route change outside this story. |
| Eval `4806ab8` | The five conflicted files take its resolution, byte-identical to `ac2ae00`'s (strict v2 reader, catalogue schema, `business-score-v2`, declared tool surfaces; the v1 fallback arm is dropped). The wrapper allowance lists all fifteen served tools. Its runner capture, launcher, CI, bundle, charter and historical-spec changes are not imported; `specs/eval-provenance/implementation.md` keeps its historical markers. |
| Local `main` `39a04d0` | Observation only, not remote freshness. |

Eventual reviewed heads of #152 and #153 are unavailable; reconciling with them
and with current `main` remains an outstanding prerequisite.

## Decision numbers

Visible branch tips allocate 0090 seven ways (including
`eval-candidate-proof`, `mcp-options-admission`, `html-client-compatibility`),
0091 eight ways, 0092 four ways (`compact-email-view`, `mcp-candidate-proof`,
`fixture-response-bounds`, `official-mail-comparison`) and 0096 twice; 0093
(`near-miss-timeouts`), 0094, 0095 (`deterministic-spam-rules`) and 0097 are
single. This stack now carries proposed 0090 (`mcp-options-admission`), 0091,
0094 and 0097. Disposition of every collision, and every acceptance, is the
operator's; nothing was renumbered, accepted or rewritten.

## Witnesses added or strengthened

| Witness | What it pins |
| --- | --- |
| `persist::tests::metadata_null_empty_present_and_corrupt_are_distinct` | Through `Persist::load`: a complete positional occurrence, `{"html":null}` and `{"text":null}` sources and a raw duplicate key each refuse as column 19, Text, with their own serde cause; null/list/number sources refuse; valid populated bytes then restore exactly. Red on the derived readers. |
| `persistence::a_corrupt_occurrence_column_refuses_to_serve` | Through real `run_on` against the data file: every shape above refuses with its own cause (not a held file); valid populated bytes then serve the focused link wait with exact provenance; SQL NULL and `[]` still serve. Red on the derived readers. |
| `mcp_revisions::focused::served_prose_states_additive_discovery` | Served llms.txt, served OpenAPI and README state thirteen preserved entries plus two appended (fifteen), and the OpenAPI/MCP-only proof split, against the live catalogue; the stale blanket claims are absent. |
| `mcp::tests::every_tool_has_exactly_one_object_output_schema` | Object-typed root and absent `$ref`, separately (393247c). |
| `api::tests::the_router_owns_one_tool_rate_across_connections_revisions_and_members` | #152's shared-budget proof; the focused waits refuse before subscription. |

## Outstanding — not claimed

- **Coordinated eval execution (AC6, T8).** The eval crate builds locked
  (`--no-run`) on this tree and its spawn-free `scoring` tests pass. The
  trial-launching regressions (`identity`, `provenance`, `wrappers` launch)
  and `focused_tools_complete_link_and_code_traps_without_guessing` were not
  run: the only mechanism here is `trial_scope`, a same-user lock, which is not
  host-wide admission, and 4806ab8's pinned launcher is not part of this
  bundle. #201 suite-wide coordination is unproved.
- **Upstream reconciliation.** Reviewed #152/#153 heads, current `main`, and
  #152's proposed 0092 corrections.
- **Inherited HIGH security-relevant review-integrity/result-attribution
  residual**, carried unchanged: the pinned engine completes instead of
  refusing the cross-attempt case ([0081](../../docs/decisions/0081-glm-flash-seats.md)
  and its must-fail witness). `has_security_residual=true`,
  `max_residual_severity=high`. No attack was reproduced, no access widened, no
  engine re-pinned; the security member owns its verdict and any hold, the
  engine owns verify and advancement.
- **Accepted costs kept:** unbounded per-mail occurrences and whole-snapshot
  link work; malformed stored metadata now stops startup until operator repair.
- **External:** engine verification and all five protected members on the
  final candidate; #151 paid baseline and actual-client proofs (operator);
  CI platform, network, chart, Docker, port-80 and #201 macOS evidence;
  operator sealing. No paid trial, push, merge, publication, tag, issue
  closure or decision acceptance was performed.
