# Plan — built-in deterministic spam signals (#260)

Spec: [spec.md](spec.md). Proposed decision:
[0122](../../docs/decisions/0122-deterministic-spam-rules.md). Base `main` 8732445.

## Technical context

- Single Rust crate. The report engine is `src/deliverability.rs`:
  `collect(raw, inputs, out)` checks `MAX_RAW_BYTES`, parses root headers with the
  bounded `headers()`, runs the demo marker, `bulk(...)` and `html::collect(raw,
  out)`, then `Collector::finish` sorts, deduplicates and enforces
  `MAX_REPORT_BYTES`. `RULE_SET_ID` (`swarmail-html/3`) is the only version and
  is part of the cache key through `Inputs.rule_set_id`.
- `src/deliverability/html.rs` is the pattern to follow for the MIME walk: one
  `MessageParser::default().parse(raw)`, an explicit stack over
  `PartType::Multipart` children building `Vec<u64>` paths, an inherited
  `excluded` flag from `attachment_name()` or an `attachment` disposition,
  `is_encoding_problem` → `Failed`, `text_contents()` → `Failed` on `None`, and the
  aggregate decoded size checked before any lexical work.
- Dependencies already present: `regex = "1"`, `mail-parser = "0.11"` (locked
  0.11.9). `src/extract.rs` sets the precedent of `LazyLock<Regex>` with
  `unwrap()` on a constant pattern.
- Every production report carries error-level `bulk.*.not_evaluated.v1`
  findings, so spam severities do not change today's predicate outcomes.

## Constitution check

| rule | how this plan holds it |
|---|---|
| 0001 accept == stored | nothing on the SMTP path or in `Store::insert` changes; spam runs only inside the lazy `Service` evaluation |
| 0002 await subscribes first | the wait loop is not edited; spam is one more collector inside the existing evaluation |
| 0003 oldest-only pruning | the collector reads the pinned `Arc<Email>` bytes only; no Store call |
| 0005 extraction at ingest | ingest extraction and stored fields are untouched (AC9 proves the message JSON is unchanged by a report) |
| accepted 0085 contract | report shape unchanged; no scores (AC10); new rule meaning ⇒ new `rule_set_id` (AC5) |
| 0045 §8 | the findings-only rule set is compatible work; weight/total wait for 0122 (AC16) |
| 0020 test placement | inline `#[cfg(test)]` in `spam.rs` (no cold path, so no `*_tests.rs`); wire tests in `tests/spam_rules.rs`; no sleeps, no wall clock, chaos not used |
| coverage 100 % exact | every arm of the catalogue `match`es and every predicate branch has an inline test; no `coverage(off)` |
| dependency policy | no manifest, lockfile or `deny.toml` change (AC13) |
| commits | unsigned, never pushed |

## Project structure (files touched)

New:
- `src/deliverability/spam.rs` — catalogue, matchers, walk, inline tests.
- `tests/spam_rules.rs` — real SMTP/REST/MCP fixtures (uses `tests/common`).
- `specs/spam-signal-rules/implementation.md` — the AC6 grep classification and
  the AC12 audit result (implementer's ledger).

Edited (one-line or one-token where possible, to limit sibling conflicts with
#253/#258):
- `src/deliverability.rs` — `mod spam;`, `RULE_SET_ID = "swarmail-spam/1"`, one
  call `spam::collect(raw, out)?;` after `html::collect`, and the id literal in the
  inline empty-report test.
- `src/deliverability/html-rules.json` — `rule_set_id` token (its loader requires
  equality with `RULE_SET_ID`); `src/deliverability/html.rs` test literal.
- `src/mcp.rs` — `MODERN_REPORT_DESCRIPTION`: new id and "spam signals" clause.
- `src/static/openapi.json` — `DeliverabilityReport` description sentence and the
  `rule_set_id` example.
- `docs/guide/deliverability.md` — new spam section; the two sentences naming the
  current id; version history.
- `src/static/llms.txt` — the same text where the guide is mirrored.
- `tests/deliverability.rs`, `tests/bulk_sender.rs`,
  `tests/mcp_revisions/structured.rs`, `tests/fixtures/html-compatibility.json`,
  `tests/fixtures/mcp-structured-output/{openapi-report-source.json,
  march-current-main.jsonl, README.md, march-current-main.provenance.json}` —
  the identity token, following the recorded "one-token edit, not a re-capture"
  precedent (`specs/mcp-report-output/implementation.md`), with a dated
  provenance note.
- `docs/decisions/README.md` — the 0122 index row.

Not touched: `Cargo.toml`, `Cargo.lock`, `evals/`, `deny.toml`, `.github/`,
`scripts/`, `.brokkr/`, `src/store.rs`, `src/smtp.rs`, `src/model.rs`.

## Research

- **Header occurrence.** `mail-parser` convenience accessors (`subject()`,
  `from()`) may resolve a duplicated header to a different occurrence than the
  first. The spec requires the first in wire order, so the implementer iterates
  the root part's parsed header list in order and takes the first
  `HeaderName::Subject` / `HeaderName::From` whose value is text / an address
  list. The duplicate-header tests in AC2 pin this whichever way the accessor
  behaves in 0.11.9.
- **Default content type.** A part without Content-Type is `text/plain`
  (RFC 2045) except inside `multipart/digest`. Confirm how 0.11.9 types such parts
  (`PartType::Text` vs `content_type() == None`) and implement eligibility as
  "`PartType::Text` and (no Content-Type or `text/plain`)", proven by the AC3
  no-Content-Type, `text/csv` and digest tests.
- **ASCII matching.** In `regex::bytes` with `(?i-u)`, case folding is ASCII-only
  and `\b` is the ASCII word boundary `[0-9A-Za-z_]`. Every phrase begins and ends
  with an ASCII alphanumeric, so `\b(?:p1|p2|…)\b` is exactly the spec's boundary
  rule. Internal spaces become `[ \t\r\n]+`; other characters go through
  `regex::escape`. Unicode-folding hazards (U+212A KELVIN SIGN folds to `k`,
  U+017F LONG S to `s`) give the non-ASCII negative fixture:
  `no credit chec\u{212A}` and `thi\u{17F} is not spam` must not match.
- **Linear cost.** `regex` guarantees O(m·n) matching with no backtracking; with
  a fixed pattern this is linear in the input. `is_match` stops at the first hit.

## Data model

```rust
#[derive(Clone, Copy)]
enum Signal { SubjectAllCaps, SubjectPunctuation, SubjectPhrases,
              FromDisplayNameAddress, BodyPhrases, BodyZeroWidth }
const SIGNALS: [Signal; 6] = [/* all six, declaration order */];
impl Signal {
    fn id(self) -> &'static str;        // exhaustive match
    fn severity(self) -> Severity;      // exhaustive match
    fn hint(self) -> &'static str;      // exhaustive match
}
const SPAM_TEXT_BYTES: usize = 4_194_304;
static PHRASES: LazyLock<regex::bytes::Regex>;   // from a const [&str; 14]
pub(super) fn collect(raw: &[u8], out: &mut Collector) -> Result<(), AnalysisError>;
```

No field exists that production code does not read (clippy `dead_code` under
`-D warnings`, and the exact coverage gate). No new public type; `Report`,
`Finding`, `Location` and `Severity` are unchanged.

## Contracts

The REST, MCP and UI report shape is unchanged. Only the `rule_set_id` value
(`swarmail-spam/1`) and the possible new `spam.*.v1` findings are new. The
OpenAPI and MCP output schemas stay closed and need no structural edit; only
descriptions and the `rule_set_id` example change. The weighted schema is a
proposal in 0122 and is not implemented.

## Approach

1. Audit first (T1): confirm on the base tree that no existing fixture would
   trigger a spam rule, so the identity token is the only expected-report edit.
2. Write `spam.rs` with its inline tests (T2–T6), wire it in and bump the id
   (T7), then make every live identity copy agree (T8).
3. Wire fixtures in `tests/spam_rules.rs` (T9–T12).
4. Guide, llms.txt, OpenAPI and MCP texts and the guide-binding test (T13).
5. Decision index row (T14), implementation ledger (T15), full gates (T16).

The collector parses once with `mail-parser`, reads Subject and From from the
root header list, walks parts exactly as `html.rs` does, collecting eligible
`(path, text)` pairs while summing their lengths against `SPAM_TEXT_BYTES`, and
only after the walk completes evaluates the four header predicates and the two
body predicates, pushing at most one finding per signal through `Collector::push`.

## Quickstart

```bash
cargo run --bin swarmail &   # SMTP 127.0.0.1:1025, HTTP 127.0.0.1:8025
printf 'From: a@example.com\r\nTo: t@example.com\r\nSubject: FREE MONEY NOW!!\r\n\r\nAct now, 100%% free.\r\n' |
  curl -s smtp://127.0.0.1:1025 --mail-from a@example.com --mail-rcpt t@example.com -T -
curl -s localhost:8025/api/v1/messages/<id>/deliverability
# rule_set_id "swarmail-spam/1"; spam.subject.all_caps.v1, spam.subject.punctuation.v1,
# spam.body.phrases.v1 among the findings
```

## Risks

| risk | mitigation |
|---|---|
| A rule fires on an existing fixture and forces edits to other stories' expected reports | T1 audits every existing fixture before code; the rules were chosen conservatively. AC12: if one fires, add the required finding and record it; never loosen the rule to hide it. |
| `mail-parser` picks a non-first header occurrence or types defaulted parts unexpectedly | Explicit wire-order iteration of the root header list; AC2 duplicate-header and AC3 no-Content-Type/digest tests pin the behavior on the locked 0.11.9. |
| Accidental Unicode semantics in the phrase regex | `(?i-u)` ASCII mode; KELVIN SIGN / LONG S negative fixtures (T4). |
| Oversized body makes analysis slow | 4 MiB aggregate cap checked before any matching; linear regex; boundary fixtures at cap and cap+1 (T6). No load test is run; none is needed for a structural bound, and none may be run unbounded (≤ nproc/4 workers, ≤ 10 min, named command, if ever). |
| Behavior change: > 4 MiB decoded text/plain now fails analysis | Stated in the guide and 0122 as an accepted cost; truncated success is refused by 0085. |
| Finding flood from many matching parts | One finding per rule (AC3's 256-part fixture). |
| Version collision with parallel siblings #253/#258 | Family-named id makes a second bump conflict textually on one line; the integrator who lands second mints a fresh, never-used id and updates the AC5 pin. A shared id across two rule sets is refused. |
| Stale `swarmail-html/3` copies in live surfaces | AC6 grep audit and its recorded classification (T8, T15). |
| Merge conflicts in shared docs (`deliverability.md`, `llms.txt`, `openapi.json`, `mcp.rs`, decisions README) | Append one guide section; one-sentence/one-token edits elsewhere; own wire tests in a new file. |
| 0122 is never accepted | The spam rules stand alone as compatible work; the guide's weight label stays accurate; #260 stays open for AC16 only. |
| Weights prematurely implemented "for later" | AC10 forbids any weight/total code; dead fields would also fail clippy and the coverage gate. |
| The coverage gate misses a branch in `Signal`'s matches or the `LazyLock` closure | The AC1 enumeration test calls every method on every signal; any report evaluation forces the closure. |
