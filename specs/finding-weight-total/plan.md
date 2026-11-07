# Plan — per-finding weight and report total (#337)

Spec: [spec.md](spec.md). Authority: accepted
[0122](../../docs/decisions/0122-deterministic-spam-rules.md) §2, read with its
2026-11-06 acceptance note and the 2026-11-07 integration note.

## Technical context

- **Language and crate:** Rust, the single `swarmail` crate. serde and serde_json
  are already dependencies. serde_json is built **without** `preserve_order`,
  which `cargo tree -e features -i serde_json` confirms on main 50095c8.
- **Owner code:** `src/deliverability.rs` holds `Finding`, `Report`,
  `Collector::{push, finish, finish_with}`, `MAX_FINDINGS = 256`,
  `MAX_REPORT_BYTES = 65_536`, `RULE_SET_ID` and `Report::qualifies`.
- **Production `Finding` construction sites.** Each gains `weight: 0`, which
  `push` overwrites:
  - `src/deliverability.rs`: the demo marker (≈879) and bulk (≈931);
  - `auth.rs` `Rule::finding` (≈224);
  - `dkim.rs` `located` (≈203);
  - `html.rs` (≈130);
  - `links.rs` (≈274, ≈380);
  - `spam.rs` `Rule::finding` (≈119).

  Test-only literals in those files and in `src/ui.rs` gain the field too.
- **The paths every report takes:**
  - `evaluate_at`, then the collectors, then `push`, then `finish`, then
    `finish_with` (cache miss);
  - the `Service` slot's cached `Arc<Report>` (cache hit);
  - `links::compose`, which re-pushes the cached and the observed findings,
    then `finish_with` (probe, never cached).

  No code outside `finish_with` builds a production `Report`.
- **Edges:**
  - REST `src/api.rs` returns `Json<Arc<Report>>`, in struct order;
  - MCP `src/mcp.rs:729` returns `text_result(serde_json::to_value(report))`,
    in sorted-key order;
  - the UI `src/ui.rs::report_section` renders fields and computes nothing.

  None of these changes, apart from schema and description text.

## Constitution check

| Rule | Status |
|---|---|
| 0001 accept == stored | Untouched. Weight lives in the lazy analysis, never on SMTP or in `Store::insert`. |
| 0002 `await` subscribes before it checks | Untouched. The predicates and wait code are not edited (AC8 proves it). |
| 0003 oldest-only pruning | Untouched. |
| 0005 extraction at ingest | Untouched. Analysis stays lazy (0085). |
| 0085 report contract | Amended exactly as accepted 0122 §2 authorizes. Bounds, ordering, cache and failure semantics are kept. |
| 0120 `dns` member | Keeps its position. `total` is appended after it. |
| 0121 §5 / 0122 §1.4 one id per rule set | `swarmail-html/8`, never shipped. |
| 0045 §8 | Compatible work under accepted records. No proposal is needed. |
| 0020 coverage placement | All new tests are inline or wire tests. No harness file, seam or `coverage(off)`. |
| Dependencies | None added. The lockfiles and `deny.toml` are unchanged. |

I found no contradiction between §2 and another accepted record. The two
places where §2 reads differently from records accepted since are settled by
the dated note. `total` after `dns` follows from §2.1 read with 0120. `/8`
follows from §1.4 and 0121 §5.

## Rulings on the panel

The simplicity and robustness positions agree on most of the design:
- weight assigned in `push`, unconditionally;
- total as a `checked_add` fold in `finish_with`, after sort and dedup and
  before the byte check;
- an inline overflow test through `Collector(vec![..])`;
- no serde default;
- no edge computation;
- predicates untouched;
- `/8`, with `total` after `dns`.

Those points are adopted as written. Where the two positions differ, the
rulings are as follows.

1. **Integer type: `u64` for both fields (simplicity), not `u32`
   (robustness).** `u64` is the type of every other wire integer in the report
   (`occurrence`, `byte_offset`, `path`), and it needs no casts. Robustness
   argued that `u32` gives a "short JSON number", but that is wrong: serde
   prints the value, not the width, so the bytes are identical. Robustness
   loses nothing it asked for. The overflow test uses `u64::MAX` instead of
   `u32::MAX`, and in-memory size grows by 4 bytes per finding.
2. **Weight rule as a private pure function `weight_of(rule_id, severity) ->
   u64`, called only from `push` (robustness).** Simplicity wrote it inline
   in `push`. §2.3 says the weight is *assigned* in `push`, and it still is.
   A named function keeps the table and its two exceptions in one readable
   place. Tests still drive `push` and never call the helper alone. The
   function returns a value and has no fallible path, so no caller branch goes
   uncovered. Cost to simplicity: one private function name.
3. **Overflow proof: the inline `finish` test, not a helper or an argument.**
   Intake's AC4 offered three proofs: a pure helper, an inline test, or an
   unreachability argument. The positions agree on the inline test, and this
   plan rules for it. Both the `None` arm of the fold and the `?` that follows
   it run in the measured report, and that stays true if `MAX_FINDINGS` or a
   weight changes.
4. **No schema `maximum` (simplicity), overruling intake's AC9 bounds of
   "0 to 10 / 0 to 2560".** §2.1 promises only "a non-negative integer", and
   0122 names per-rule overrides as an additive later change. A `maximum` would
   be a second copy of the severity table and of `MAX_FINDINGS × 10`, so that
   later additive change would break the schema. The current values go into the
   property descriptions and the guide instead. Cost: a client that reads only
   the schema keywords cannot learn the 2,560 bound. The bound is still
   enforced by the code and proven by AC3.
5. **MCP keeps its sorted-key serialization. This rejects robustness's demand
   that MCP emit struct order.** Robustness asked that MCP not "hand-build" a
   map, because a map would sort the keys. MCP already serializes the `Report`
   through `serde_json::to_value`, and it already sorts the keys. Every MCP
   transcript is in that order, including the pinned March fixture. Switching
   MCP to struct order would move existing keys and break §2.1's "existing key
   prefixes stay byte-identical" on that edge. In sorted order, `weight` (after
   `severity`) and `total` (after `rule_set_id`) already come last. So "last on
   every edge" holds with no change to MCP code, and AC9 pins it per edge.
   Robustness's underlying concern was one `Report`, one serializer per edge,
   and no edge computing anything. That concern is met. Cost: REST and MCP order
   keys differently, as they do today. Byte identity is per edge (AC6), and the
   two edges agree on values. This reading should be stated in the 0122 note
   (see the record change below).
6. **The UI does not show weight or total (simplicity).** §2.3 forbids the edge
   from computing a score, but it does not require a display. Showing it later
   is additive. Robustness did not ask for a display.
7. **Served text, by status and not by number range.** The task names 0119–0125.
   0126 was accepted on 2026-11-07, and the same served sentences also name
   other decisions. So the rule is: no surface this story edits calls an
   accepted decision "proposed", and the check reads each decision file's
   status line. A test enforces this for `openapi.json` and the MCP
   descriptions, where no dated history exists. llms.txt and the guide are
   edited by hand and keep their dated history lines.
8. **Robustness's failure-mode table.** It covers a crash between accept and
   commit, a hanging webhook, the cap race, chaos, hostile DATA, cert expiry,
   parallel sessions and a stale cache. Each mode is answered by structure
   that already exists, with no new mechanism:
   - analysis stays off the ingest path;
   - the slot caches only an `Ok` report;
   - weight is code-owned and senders cannot reach it;
   - `/8` changes the cache key's rule-set member.

   None of these modes gets new code. Each AC that touches one cites the
   existing test it extends.

## Approach

### Data model (`src/deliverability.rs`)

```rust
pub struct Finding { rule_id, location, severity, fix_hint,
    /// Set by `Collector::push` from the severity (0122 §2.2); a collector's value is replaced.
    pub weight: u64 }
pub struct Report { rule_set_id, findings, dns,
    /// Sum of the findings' weights, computed in `finish_with` (0122 §2.3).
    pub total: u64 }
```

- The derives are unchanged. Declaration order is the REST wire order.
- The derived `Ord` still sorts by the first four fields first. `weight` is a
  function of `rule_id` and `severity`, so for pushed findings the sort and
  `dedup` results are unchanged.

### Weight

```rust
fn weight_of(rule_id: &str, severity: Severity) -> u64 {
    if rule_id == "demo.marker.v1" || rule_id.contains(".not_evaluated.") { 0 }
    else { match severity { Info => 1, Warning => 3, Error => 10 } }
}
fn push(&mut self, mut finding: Finding) -> Result<(), AnalysisError> {
    if self.0.len() == MAX_FINDINGS { return Err(AnalysisError::Failed); }
    finding.weight = weight_of(&finding.rule_id, finding.severity);
    self.0.push(finding); Ok(())
}
```

Branch coverage needs these inputs:
- demo (the `||` short-circuit is taken);
- a non-demo id with `.not_evaluated.`;
- an id that is neither;
- each of the three severities.

AC1 and AC2 exercise all of them.

### Total (`finish_with`)

```rust
self.0.sort(); self.0.dedup();
let total = self.0.iter().try_fold(0u64, |sum, f| sum.checked_add(f.weight))
    .ok_or(AnalysisError::Failed)?;
let report = Report { rule_set_id, findings: self.0, dns, total };
if serde_json::to_vec(&report).unwrap().len() > MAX_REPORT_BYTES { return Err(..) }
```

### Contracts

- **OpenAPI** (`src/static/openapi.json`):
  - `DeliverabilityFinding` gets `required += "weight"` and the property
    `weight: {"type":"integer","minimum":0,"description":"0122 §2: info 1, warning 3, error 10; 0 for ids containing .not_evaluated. and for demo.marker.v1"}`;
  - `DeliverabilityReport` gets `required += "total"` and the property
    `total: {"type":"integer","minimum":0,"description":"Sum of the findings' weights; at most 2560"}`;
  - the route and report descriptions replace "no weight or total", move the
    example id to `/8`, and stop calling accepted decisions proposed.
- **MCP** (`src/mcp.rs`):
  - the `finding` output object gains `"weight":count`, and the report object
    gains `"total":count` (the existing `{"type":"integer","minimum":0}`);
  - both lists of required keys gain the new key;
  - `MODERN_REPORT_DESCRIPTION` moves to `/8` and states the weight and total.
- **Pinned schema source:**
  `tests/fixtures/mcp-structured-output/openapi-report-source.json` gets the
  same structural edit. `served_openapi_pins_and_negative_controls` gains a
  row that drops each property from `required` and must fail.

### Record change

The 2026-11-07 integration note on 0122 is already in the worktree from intake,
uncommitted. Implement commits it with the first code commit. Implement also
appends one sentence to that note: MCP renders the same report through
serde_json's sorted map, where `weight` and `total` also sort last and no
existing MCP byte moves. No body edit, no status change, no 0085 edit. In
`specs/spam-signal-rules/spec.md`, AC16 gains one dated line that points to
this spec. Its text is not rewritten.

### Served text and guide

- `docs/guide/deliverability.md`:
  - the six "(proposed in 0122 — not in the report)" labels become plain
    weights;
  - the paragraph near line 988 that says reports carry no weight is replaced
    by a *Weight and total* subsection;
  - the "Signals counted elsewhere" sentence moves into the present tense;
  - the current-id sentence moves to `/8`, and the history lines stay.
- `src/static/llms.txt`: the same facts.
- `spam.rs:692`: the guide-phrase assertion follows the new text.

### Project structure

No new production file. The tests extend these existing suites:
- the inline tests in `src/deliverability.rs` and `src/deliverability/links.rs`;
- `tests/deliverability.rs`;
- `tests/mcp_revisions/structured.rs`;
- `tests/spam_rules.rs` and `tests/bulk_sender.rs` (expectations only).

### Research

- serde_json features: no `preserve_order`, so MCP sorts keys.
- `Report` deserialization happens only in tests. Production never reads a
  stored `Report`.
- The slot caches `Option<Arc<Report>>`, and only an `Ok` reaches it.

### Quickstart (manual check)

`cargo run -- serve`, then send one mail with `X-Swarmail-Demo: warning`. Then
`curl /api/v1/messages/<id>/deliverability`. The body ends with
`,"total":N}`, and the demo finding ends with `,"weight":0}`.

## Risks

1. **Reports near the byte ceiling now fail.** Each finding grows by 11–12
   bytes and the report by up to 14. A message whose report was just under
   65,536 bytes may now return `AnalysisError::Failed`. §2.3 intends this,
   because the bound must count what is served. *Mitigation:* AC5 proves the
   exact edge, and the guide and llms.txt state the headroom cost.
2. **A collector that forgets the placeholder fails to compile.** That is
   intended. A placeholder that a refactor carries past `push` would ship
   wrong weights. *Mitigation:* `push` overwrites unconditionally, and AC1's
   `weight: 99` fixture proves it.
3. **Test-built `Collector(vec![..])` bypasses `push`.** Such findings keep
   their placeholder weights. *Mitigation:* the weight criteria (AC1–AC3, AC5)
   build through `push`. Only the overflow and canonicalization tests build
   directly, and they say why.
4. **Mechanical churn in expectations.** Many `json!` expectations gain
   `"weight":N`, and `/7` moves to `/8`. A careless global replace could
   rewrite dated history. *Mitigation:* move only live pins. Search for
   `swarmail-html/7` and check each hit for a date. AC11 lists which surfaces
   move.
5. **MCP and REST key order differ.** A reviewer may read "last" as struct order
   on every edge. *Mitigation:* ruling 5, the sentence added to the 0122 note,
   and the per-edge raw-byte tests in AC9.
6. **The pinned March fixture is hand-edited.** A hand edit can drift from the
   served bytes. *Mitigation:* the existing structured suite compares the row
   with the served report. The new row is copied from served bytes, and the
   sha256 and dated provenance are recomputed. The old row is not kept as an
   accepted alternative.
7. **The served-text status test could flag dated history.** *Mitigation:* it
   scans only `openapi.json` and the MCP descriptions, which carry no history.
   llms.txt and the guide are edited by hand, with history kept.
8. **The coverage gate is exact.** *Mitigation:* every new branch has a named
   inline test (see tasks), and there is no new file or seam. Run
   `scripts/coverage-gate.sh` and its selftest before reporting.
