# Metric parser proof

Date: 2026-10-24. Epic #192, issue #202. Status: draft specification.
Baseline: `6696a48e7271b7aaaea8bdebc525be19cd24fbf3`, tree
`d34bfadbbf8602f20847fec1d003a52e445d8a1f`.

## Feature and rationale

Establish whether the existing decoded CR/NUL/C1/bidi request labels survive
an actual HTTP scrape and an independently sourced upstream Prometheus parser
with their exact identities intact. Add only the missing regression and proof
if the current renderer conforms. This is evidence gathering under accepted
[0065](../../docs/decisions/0065-decoded-inbox-metrics.md), not a new runtime
contract or acceptance of the outstanding security residual.

The supplied [Prometheus exposition reference](https://prometheus.io/docs/instrumenting/exposition_formats/)
says label values may contain any UTF-8 sequence and backslash, double quote
and LF must be escaped. That task-supplied statement supports the expected
behavior, but does not prove transport/parser acceptance. No independent web
retrieval is claimed. The existing held CR/NUL test uses the repository's
reader; its passing result cannot substitute for this independent proof.

Accepted [0043 §6](../../docs/decisions/0043-mcp-origin-boundary.md) specifies
exactly those three escapes and literal passage otherwise. Accepted 0065
requires selecting the segment before one strict UTF-8 decode, retaining the
whole original segment on invalid UTF-8. This design preserves both rulings
and changes no public contract or frozen guarantee; no numbered decision is
declared or reserved. Concrete counterevidence takes the proposal-and-stop
branch below, not an implicit amendment.

## Evidence and unresolved prerequisite

Read both run-local positions, current intake
`.forge/tasks/epic-192-issue-202-residual-re-e-7d8e6287.md`, stopped triage
`.brokkr/state/tasks/triage-first-continue-epic-192-i-72123301.md` and result
`.forge/results/f874c220-a8fa-4ac3-b56c-d8f088da2430.json`.
The full recovered packet retains these historical aggregate events:

- 581 / `a0d79d66efad4f680c3f1e4f3690c7803227605d4b26fbd42ab91a221b186717`.
- 796 / `d6640e1b01d6c3d364af29f1b062bbef0e629b518a9a1d432aa5484d41fdf9d8`.

Both carry `has_security_residual=true`, `max_residual_severity=low`,
`fixes_applied=false`, `spec_defect=false`. All eight member notes were read.
The newer five-role panel `ce193b6b-3136-4f1a-a56f-08334eea4bc4`, recovered
under `.forge/epic192/issue202/recovered-review-evidence/`, retains those facts
in every member on this baseline. Its copied inspection records seq 625,
`REVIEW-RESIDUAL-UNSAFE`, hard stop. All ten original receipt hashes match
their inventories. Copy integrity does not authenticate an unavailable source
journal; neither journal database was queried here.

Preserve original notes, completed documentation repairs, draft/acceptance
provenance, the finite manual 0065-to-0043 pointer audit, unrelated 0017
numbering ambiguity, historical environment restrictions, harness filename
limits and the possible concurrent-fixture false failure. New evidence is
appended and candidate-bound, never substituted into those historical records.

**Upstream compatibility remains unproved.** The chief confirmed that
`promtool`, `prometheus` and `go` are absent from PATH and Python cannot import
`prometheus_client`; bounded filename checks under `.forge/bin` and `target`
found no suitable parser artifact. This is not an exhaustive machine search.
There is no executable upstream-parser command to claim today. AC3 is an
explicit prerequisite to a successful proof, not a fabricated invocation or
an invitation for hermetic verify to install tools. Drafting this plan does
not establish executable readiness or permit delivery while it is unresolved.

## Panel synthesis and scope

Adopt simplicity's zero-runtime-change ruling, existing cold-path harness,
one owning dated receipt and sequential, exclusive fixtures. Reject a new
renderer, parser framework, crate, dependency, registry, route, configuration
or sanitizer. The cost is retaining potentially awkward raw control values
and deferring any incompatible remedy to the operator.

Adopt robustness's complete scalar matrix, complete byte-preserving HTTP
capture, versioned upstream parser, negative controls and final-candidate
review. Reject the smaller interpretation that another local grammar pass
or a standards quote suffices. That would leave the exact residual untested;
the chosen proof costs capture plumbing, tool provenance and a larger matrix.
Both positions actually require independent parsing; there is no runtime
policy disagreement to average.

Rule for the existing sequential owner over default process isolation. Narrow
the broad CR/NUL fixture filter only if needed, without losing exact sample
checks. The losing default-isolation option avoids some shared-state audit
but adds process machinery; the chosen option costs an executable-wide
fixture/headroom audit. Isolation is permitted only for demonstrated conflict.
Rule for one observed format and named upstream implementation over universal
scraper certification: bounded evidence cannot guarantee every parser,
proxy, terminal or OpenMetrics version. Any unsupported behavior is disclosed.

No normalization, re-encoding, extra escapes, dropped rows, valid-UTF-8 raw
fallback, aliases or post-validation accounting. Preserve routing, admission,
RAII ownership, total accounting including refusals/scrapes, family names,
MCP identity, cumulative 4096-name awaits cap and reserved LF overflow. Live
request rows remain bounded by activity rather than that cumulative cap.
Preserve alias coalescing, plus, malformed percent, case/padding/Unicode,
encoded slash as data, single decoding and the `%FF`/`%25FF` collision.
Metrics are activity, not admission or storage receipts; no cross-family
atomic-snapshot promise is added.

## Acceptance Criteria

1. **AC1 — Evidence fidelity.** A dated receipt in
   [the owning plan](../decoded-inbox-metrics/plan.md) preserves the events,
   notes and limits above, records commands/exits, candidate HEAD/tree,
   versions/digests and byte-safe evidence paths, and distinguishes new runs
   from historical passes. All security facts remain true/low until fresh
   protected review supplies a policy-admissible disposition.
2. **AC2 — Exact input and wire matrix.** Test CR U+000D, NUL U+0000, all
   32 C1 scalars U+0080–U+009F, and bidi U+061C, U+200E–U+200F,
   U+202A–U+202E, U+2066–U+2069 separately and in a combined value.
   Retain the original CR/NUL pair. Add ordinary Unicode and backslash,
   quote and LF controls, distinguishing LF from literal backslash-n.
   Percent spellings encode valid UTF-8, not lone high bytes. Record each
   raw selector, decoded UTF-8 hex/code points, expected status/body, and
   complete real scrape status, headers/Content-Type parameters, framing
   and body bytes. HTTP framing is removed exactly; body bytes are neither
   trimmed, Unicode-line-split, NUL-truncated nor newline-normalized.
3. **AC3 — Independent format and identity proof.** First identify an available
   offline upstream parser/scraper artifact and record its exact invocation,
   version/source/digest, selected format and any explicit fallback in the
   plan before claiming implementation/verification of this criterion. Feed
   each unmodified complete captured body to it. Format selection must match
   the observed response; do not replace its Content-Type. If a real scraper
   requires fallback, prove and disclose that exact configuration and its
   limits; an unexplained media-type mismatch is unresolved counterevidence.
   Every case must parse completely and yield exactly one attributable
   request sample, value 1, whose label bytes equal the decoded identity,
   with no raw alias, truncation, normalization or additional fixture sample.
   Check the complete sample set for unexpected fixture-bearing families or
   rows, not merely a selected line. Repository grammar, lint-only success
   and standards text are insufficient. Missing tooling leaves AC3 unproved.
4. **AC4 — Existing lifetime and admission.** The real router produces its
   response before the request-owned hold signals readiness; both guards
   stay live across the scrape without a held registry lock. CR/NUL/C1 keep
   exact 400/body bytes and never register active/cumulative awaits. Bidi and
   other admitted fixtures retain their actual successful read behavior,
   with no invented control refusal. Release, join and subsequent scrape
   prove request-row removal. Retain the two real parked encoded space/
   Unicode awaits, their same-scrape six rows, successful joins, live cleanup
   and cumulative retention. Preserve hold drop/cancellation tests and their
   stated limits. Exclusive fixtures, bounded tasks and cap headroom pass
   under the normal parallel suite without sleep synchronization or resets.
5. **AC5 — Falsifiable proof.** Positive controls pass the upstream parser.
   Bounded malformed syntax controls fail it; changed/truncated expected
   labels and an added fixture sample fail the identity/count comparator.
   Record a controlled assertion-red and restored-green for the new property;
   tool absence, timeout and build failure do not qualify. Preserve existing
   invalid/mixed UTF-8 and old-helper sensitivity evidence. Keep production
   rendering/decoding unchanged when conformant.
6. **AC6 — Counterevidence stops.** If an exact case is rejected, invalid,
   ambiguous, changes identity or adds samples, write the unnumbered
   `.forge/epic192/issue202/operator-proposal.md`, marked PROPOSED / NOT
   ACCEPTED, with exact bytes, format/configuration, parser provenance,
   command, observed outcome, affected 0065/0043 clauses, precise proposed
   amendment, alternatives and identity/coalescence/refusal consequences.
   Stop before contradictory implementation or policy edits. Missing tools
   alone are not a failing-case receipt and do not justify an invented
   amendment. The proposal branch is not a successful compatibility proof.
7. **AC7 — Verification.** Source/test changes pass the targeted suites in
   [plan.md](plan.md) and the entire authoritative
   [house gate inventory](../../docs/house-rules.md#gates--part-of-every-change)
   on the final candidate, including pinned tooling/shims, exact zero-missed
   coverage and selftest, publish/seal proofs, licenses and fail-closed
   native-tls absence. CI-owned proofs stay distinguished; unavailable gates
   are outstanding, not historical passes. No benchmark claim is introduced.
8. **AC8 — Protected disposition.** Require five separate candidate-bound
   results under accepted [0058](../../docs/decisions/0058-codex-review-routing.md):
   Astra correctness, official DeepSeek Flash security, Sonnet 5.5 tests,
   Astra guarantees and independent Fable 5.1. No Pro, fallback, combined
   Astra result or smaller route. Preserve worst verdict, security-hold and
   [0050](../../docs/decisions/0050-delivery-gates.md) refusal of unsafe or
   unknown residual facts. No compatibility or gate result itself clears
   security. No push, merge, tag, publication, issue closure or decision
   acceptance; commit unsigned under 0056 if policy reaches ship.

Frozen synchronous accept==stored, subscribe-before-scan and oldest-only
pruning, plus ingest-time extraction, remain unchanged in every branch.
