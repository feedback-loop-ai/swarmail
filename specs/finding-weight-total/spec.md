# Per-finding weight and report total (issue #337)

- **Epic:** #251. Follow-up to #260, which built only §1 of 0122.
- **Run:** `issue-337-emit-the-per-finding-w-5aed52dd`, `claude-careful`, from
  `main` `50095c8` (which carries #255, SPF and DMARC, as #343).
- **Authority:** accepted [0122](../../docs/decisions/0122-deterministic-spam-rules.md)
  §2 ("Weight and total"), accepted by the operator on 2026-11-06. Under 0122's
  Disposition, that acceptance authorizes this follow-up. It amends accepted
  [0085](../../docs/decisions/0085-deliverability-report.md)'s report contract only
  as 0122 §2 says. Read with accepted
  [0120](../../docs/decisions/0120-deliverability-dns-policy.md) (the `dns`
  member), [0121](../../docs/decisions/0121-deliverability-link-checks.md) §5 and §6
  (rule-set ids, the probe composition),
  [0125](../../docs/decisions/0125-offline-dkim-verification.md) (DKIM) and
  [0126](../../docs/decisions/0126-spf-dmarc-alignment.md) (SPF/DMARC, accepted
  2026-11-07). This is compatible work under accepted records. No new decision
  number is drafted and no decision status changes.
- **Record change:** one dated integration note on 0122 (2026-11-07), a pointer
  and not a body edit. It settles the two points that 0122's 2026-11-06 note
  left to this follow-up: `total` comes after `dns`, and the id is
  `swarmail-html/8`.

## Feature and rationale

0122 §1 shipped six spam signals, and the report already carries findings from
the demo marker, the bulk checklist, the HTML heuristics, the link rules, DKIM,
SPF and DMARC. A reader who wants one number to compare two messages, or to
watch a template drift, still has to invent a weighting. Two readers then
invent two different ones, and the edge they read from (REST, MCP, the UI)
becomes a place where scores are computed. 0122 §2 settles a single,
severity-derived weighting inside the report evaluator. The number is
therefore part of the versioned, cached and bounded report bytes, and no edge
ever computes it:

- Every finding gains `weight`, a non-negative integer, as its **last** key.
- The report gains `total`, the sum of its findings' weights, as its **last**
  key, after 0120's `dns` member.
- Weight depends on severity alone: `info = 1`, `warning = 3`, `error = 10`.
  Any rule id containing `.not_evaluated.` weighs 0, because missing trusted
  evidence is not a sender signal and a fresh deployment must not score every
  message. `demo.marker.v1` weighs 0 because it is synthetic. There is no
  per-id table, so all rule families count the same way: demo, bulk, HTML,
  links, spam, DKIM, SPF and DMARC.
- The weight is informational. No predicate reads it. `no_findings_at_or_above`
  and `Report::qualifies` keep deciding on severity, plus 0120's `temp_fail`
  rule, exactly as today.

The report bytes change, so the rule set takes a new, never-shipped id.
Main is `swarmail-html/7` (#255), so this story integrates as
**`swarmail-html/8`** (0121 §5, 0122 §1.4). The `swarmail-spam/2` named in
0122 §2.5 does not apply, because the families share one id.

### What the change does not do

- It touches nothing on the SMTP path, in `Store::insert`, in `await`'s
  subscribe-then-check, in pruning or in ingest extraction (0001–0003, 0005).
  Weight is computed in the lazy analysis, from stored bytes.
- It adds no predicate, threshold, query parameter or configuration knob that
  reads weight or total (0122 §2.4).
- The HTML UI does not show the weight or the total. It renders the same
  `Report` and computes nothing.
- It adds no per-rule override, no dependency and no new production module.

## Contract

### Wire shape

REST serializes the `Report` struct, so the key order is the declaration order:

```text
finding: {"rule_id":…,"location":…,"severity":…,"fix_hint":…,"weight":N}
report:  {"rule_set_id":"swarmail-html/8","findings":[…],"dns":{…},"total":T}
```

MCP serializes the same `Report` through `serde_json::Value`. The crate builds
serde_json without `preserve_order`, so MCP's text block and `structuredContent`
put keys in sorted order, as they do today. In sorted order the new keys also
come last: `fix_hint, location, rule_id, severity, weight`, and
`dns, findings, rule_set_id, total`. On each edge, every byte that came before
the new key is unchanged. That covers each finding before `,"weight":N` and the
report before `,"total":T`.

### Rules

1. **Assignment.** `Collector::push` sets `weight` on every finding it accepts,
   after the `MAX_FINDINGS` check. It replaces whatever value the collector
   wrote. No other code assigns a weight.
2. **Total.** `Collector::finish_with` sorts and deduplicates as today. It then
   computes `total` as a `checked_add` fold over the remaining findings, starting
   at 0, and builds the `Report`. The `MAX_REPORT_BYTES` check runs on that
   report's bytes, so it counts every `weight` and the `total`. Overflow returns
   `AnalysisError::Failed` and never wraps or saturates. With today's bounds,
   `total ≤ MAX_FINDINGS × 10 = 2,560`.
3. **One source of bytes.** A cache hit returns the cached `Arc<Report>` and a
   miss returns a freshly finished one. Both are serialized by the same
   serializer. The probed response of 0121 §6 (`reachability=true`) re-pushes
   the cached findings and the observed findings through `push` and
   `finish_with`, so it weighs and totals the union. REST, MCP and the UI
   compute nothing.
4. **Required and closed.** `weight` and `total` are required on deserialize,
   with no serde default. `deny_unknown_fields` stays.
5. **Schemas.** OpenAPI `DeliverabilityFinding` and `DeliverabilityReport` and
   the MCP output schema of `swarmail_get_deliverability_report` each gain the
   new property as required: `{"type":"integer","minimum":0}`.
   `additionalProperties: false` stays. The schemas set no `maximum`, because
   §2.1 promises only "a non-negative integer". The descriptions state the
   current values instead: weight is one of 0/1/3/10, and total is at most
   2,560.

## Acceptance Criteria

Each criterion has a passing fixture and a failing one, meaning a wrong
implementation the test refuses. AC1–AC9 together are **AC16 of
[`specs/spam-signal-rules/spec.md`](../spam-signal-rules/spec.md)**, the
criterion held until 0122 §2 was accepted.

- **AC1 — Severity weights, assigned in `push`.** A finding pushed through
  `Collector::push` weighs 1 at info, 3 at warning and 10 at error. The proof
  covers findings from at least three rule families (for example `spam.*`
  warning, `html.*` info, `bulk.*` error) and reads the weight both on the
  struct and in the serialized JSON. A finding that a collector builds with
  `weight: 99` comes out of `push` at its severity weight.
  *Fails:* an error finding at 3, an info finding at 0, or a placeholder weight
  that survives `push`.
- **AC2 — Zero exceptions.** Any id containing `.not_evaluated.` weighs 0 at
  info, warning and error, including a real `bulk.*.not_evaluated.v1` error from
  the evaluator. `demo.marker.v1` weighs 0 at info, warning and error.
  *Fails:* the neighbouring id without `.not_evaluated.` (for example
  `bulk.x.v1` at error) weighs 10 and must not be 0. A report that holds a
  demo error and a real error totals 10, so an implementation that zeroes
  everything is refused.
- **AC3 — Total is the sum after deduplication.** `total` equals the sum of the
  serialized findings' weights. An exact duplicate pushed twice appears once and
  counts once. 256 distinct error findings pushed through `push` give
  `total == 2560`. An empty report has `total == 0`.
  *Fails:* a total equal to the sum before deduplication.
- **AC4 — Overflow fails, never wraps.** `Collector(vec![w = u64::MAX, w = 1])`
  through `finish` returns `Err(AnalysisError::Failed)`. The test runs inline in
  `src/deliverability.rs`, with no seam, no harness file and no `coverage(off)`.
  The exact coverage gate stays at 100%.
  *Fails:* `Ok` with total 0 (a wrapped sum) or with `u64::MAX` (a saturated
  one).
- **AC5 — The byte bound counts weight and total.** A report built through
  `push` with weight-10 findings, whose compact JSON is exactly 65,536 bytes
  including every `"weight":10` and the `"total":…`, finishes. One more byte
  fails. Both existing exact-boundary tests compute their overhead from the
  serialized report, never from a hardcoded constant.
  *Fails:* a report that fits only when the new keys are left out of the count.
- **AC6 — Cache hit and miss are byte-identical.** Inline: on one `Service`, a
  miss and then a hit (`Arc::ptr_eq`), plus a miss on a fresh `Service` for the
  same mail, all serialize to identical bytes that contain `weight` and `total`.
  On the wire: two REST GETs of one report return identical bodies, and two MCP
  `swarmail_get_deliverability_report` calls return identical text blocks.
  *Fails:* any byte differs.
- **AC7 — The probe composition weighs and totals.** The probed response's
  observed `link.*` findings carry their severity weights, and its `total` is the
  sum over the composed, deduplicated union. The deterministic report it
  composes from keeps its own total.
  *Fails:* a composed total that equals the cached report's total although
  weighted findings were observed.
- **AC8 — Predicates ignore weight and total.** Inline: a report of 256 info
  findings (total 256) qualifies at warning. A report with one warning (total 3)
  refuses at warning. A report whose only finding is a zero-weight error
  (total 0) refuses at error. A `temp_fail` report with total 0 refuses at info.
  On the wire, over REST `assert`/`await` and MCP `swarmail_wait_for_email`
  with `no_findings_at_or_above`: two messages whose reports have **equal
  totals** get different verdicts by severity. For example, the passing gate
  message of #255 and the same message with `X-Swarmail-Demo: error`, which
  weighs 0. `Report::qualifies`, `Store::qualify` and `Store::wait_for_report`
  are unchanged, and a source scan proves that `Report::qualifies` does not
  mention `weight` or `total`.
  *Fails:* a total-based shortcut such as "total 0 ⇒ clean" or "total above N ⇒
  dirty".
- **AC9 — Key order on the wire.** Over real SMTP and then REST, the raw body
  starts with `{"rule_set_id":"swarmail-html/8","findings":[` and ends with
  `},"total":T}` after the `dns` object. Every finding's raw text ends with
  `,"weight":N}`. With the new keys removed, the body is byte-identical to the
  body the test pinned before this change. The same holds over MCP on
  2025-03-26 (text), 2025-06-18 and 2025-11-25 (text and `structuredContent`),
  in sorted-key order. The proof uses raw-byte prefix and suffix checks, because
  `Value` sorts keys and cannot witness order.
  *Fails:* `total` before `dns`, `weight` anywhere but last, or a moved existing
  key.
- **AC10 — Schemas.** In `src/static/openapi.json`, `DeliverabilityFinding`
  requires `weight` and `DeliverabilityReport` requires `total`. Both are
  `{"type":"integer","minimum":0}` and listed last, with
  `additionalProperties: false` kept. The MCP output schema matches, and the
  pinned `openapi-report-source.json` matches. A negative control that drops
  either property from `required` fails the existing pin test. `Report` and
  `Finding` refuse to deserialize JSON that is missing `weight` or `total`, or
  that has an extra key.
- **AC11 — Rule-set id and record.** `RULE_SET_ID == "swarmail-html/8"`, and
  every live pin moves with it: test expectations, the html catalogue snapshot
  if a test ties it to `RULE_SET_ID`, route and schema descriptions,
  `MODERN_REPORT_DESCRIPTION`, llms.txt and the guide's current-id sentence.
  Dated history lines keep their ids. The dated 2026-11-07 integration note on
  0122 is committed. No decision's status line changes, and 0085 is not edited.
- **AC12 — Served text.** In `src/static/openapi.json`, `src/static/llms.txt`,
  the MCP tool descriptions, the guide's current-state prose and the source
  comments under `src/deliverability*`, no accepted decision is called
  "proposed". The check covers 0119–0126 and any other decision those
  surfaces name. A test reads each `proposed NNNN` occurrence in `openapi.json`
  and in the MCP tool descriptions, then reads the status line of
  `docs/decisions/NNNN-*.md`, and fails if that status is accepted. The
  guide's six "Weight: N (proposed in 0122 — not in the report)" labels become
  "Weight: N". The guide's statement that reports carry no weight or total is
  replaced by a short *Weight and total* section covering:
  - the table and the two exceptions;
  - that predicates ignore weight;
  - the 2,560 bound;
  - the headroom cost: about 11–12 bytes per finding and up to 14 for the total.

  llms.txt carries the same facts. The guide's dated rule-set history stays
  history. The test that pins the old guide phrase (`spam.rs`) pins the new one.
- **AC13 — Pinned fixtures.** The seed report row (line 8) of
  `tests/fixtures/mcp-structured-output/march-current-main.jsonl` is replaced by
  the bytes this tree serves for the same seeded mail: `/8`, weights and total,
  in sorted-key order. This is a hand edit of that one row, not a re-capture.
  `fixture_sha256` is recomputed. One dated 2026-11-07 (#337) entry is appended
  to the provenance's `updated` string, and one to the fixture README, both in
  the established style. Every other fixture that records report bytes or a
  digest of them is recomputed in the same way. None is loosened to accept the
  old bytes as an alternative.
- **AC14 — Gates.** Every house gate passes, `--locked`:
  - fmt;
  - clippy;
  - tests;
  - the binary build;
  - the evals crate's fmt, clippy and tests against the candidate binary;
  - the coverage gate at exact 100% and its selftest;
  - the publish-guard and seal proofs;
  - `cargo deny check licenses`.

  The brokkr bundle compile runs in verify if the sandbox has no brokkr. No
  dependency is added. `Cargo.lock`, `evals/Cargo.lock` and `deny.toml` are
  unchanged.

## Out of scope

Per-rule weight overrides (0122 names them an additive later change, with
their own id). Any predicate, threshold or sort order that reads weight or
total. Showing weight or total in the HTML UI. Changing how MCP serializes
(sorted keys stay). A release, release notes or a version bump. Load or
throughput proofs: none is needed, because the change adds one O(≤256) fold
per report.
