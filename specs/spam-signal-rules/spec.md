# Built-in deterministic spam signals — issue #260

Date: 2026-11-03. Base: `main` 8732445. Run: `issue-260-built-in-deterministic-9e9f4283`
(claude-careful). Epic #251, phase 3, priority P2.

Decision: 0122 — [Deterministic spam-signal rules, and a proposed weight and total](../../docs/decisions/0122-deterministic-spam-rules.md), accepted (operator, 2026-11-06).

Authorities (accepted, cited, not declared): [0085](../../docs/decisions/0085-deliverability-report.md)
and its [report contract](../deliverability-report-contract/spec.md#contract);
[0089](../../docs/decisions/0089-bulk-sender-checklist.md) (bulk checklist);
[0106](../../docs/decisions/0106-html-client-compatibility.md) and
[0107](../../docs/decisions/0107-html-review-repair.md) (HTML heuristics);
0045 §8 as quoted in [house rules](../../docs/house-rules.md) (code under a
proposed decision); [0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md)
(test placement and writing disciplines).

## Feature

Add a third built-in collector to the lazy deliverability report: an offline,
deterministic, compile-time rule set of six **spam signals** that read only the
captured message (its root Subject, its root From and its sender-supplied
`text/plain` bodies). Each signal emits at most one typed finding — stable
`spam.*.v1` rule id, severity, location and a fixed fix hint — into the existing
report, which is computed lazily from the stored bytes, cached, and never touched
by SMTP, ingest or the Store (0001, 0005, 0085 §3). The report's single
`rule_set_id` changes to **`swarmail-spam/1`**, so the version is recorded in
every report. The guide documents the rule set rule by rule.

## Why

Epic #251 asks swarmail to tell a developer, before production, why their mail
looks like spam. The operator ruled on 2026-10-26 (quoted in 0085's Authority
line) that this is built-in and deterministic, with no external scorer. Signals
visible in the captured bytes are cheap, reproducible across runs and machines,
and need no network, clock, locale or dependency. The bulk-sender checklist
(#257) and the HTML heuristics (#259) already cover structure, authentication and
rendering; this story adds the content-pressure signals they deliberately do not.

## The weight and total: held, not shipped

The issue asks for findings "with weights and a total". The accepted report
contract says a report is `{rule_set_id, findings}`, each finding has *exactly*
`rule_id`, `severity`, `location`, `fix_hint`, and "no … scores … enter the
report". A weight and a total amend that accepted ruling, so under 0045 §8 they
wait for the operator's acceptance of 0122. **This run ships no weight or total —
not in the JSON, not as a dormant field, not behind a cfg.** 0122 proposes the
exact scoring model and schema so acceptance is a small additive follow-up (one
version bump, `swarmail-spam/2`), and states how bulk and HTML findings count.
The guide shows each rule's *proposed* weight, labelled as not emitted. Issue
#260 stays open for that part; the delivery must not report the issue complete.

## The rule set (normative)

Common definitions:

- **Subject** — the first root `Subject` field in wire order (not whichever
  occurrence a library accessor prefers), RFC 2047-decoded and unfolded by
  `mail-parser`. Missing, or a value that is not text, means no Subject finding.
- **From mailbox** — the first mailbox of the first root `From` field in wire
  order, display name RFC 2047-decoded. A group, an empty list or a mailbox
  without an address yields no From finding.
- **Eligible text part** — a non-multipart MIME part reached from the root
  through multipart containers only, whose content type is `text/plain` or absent
  (RFC 2045 default; a `multipart/digest` child's default is `message/rfc822` and
  is not eligible), and which neither is nor descends from a part with an
  attachment disposition or a filename/name parameter. Embedded `message/*`
  bodies are opaque and never eligible. `text/html` is never eligible: HTML is
  #259's surface, and an HTML-only message already receives
  `bulk.plain_text.v1`. Its text is the transfer- and charset-decoded contents;
  a detected encoding problem or undecodable eligible part fails the report.
- **Letters** are ASCII `A–Z`/`a–z` only; case comparison is ASCII-only. No
  Unicode case folding, locale, clock or network enters any rule.
- **Phrase list** (fixed, 14 entries): `act now`, `limited time offer`,
  `100% free`, `risk-free`, `risk free`, `you have won`, `you are a winner`,
  `claim your prize`, `no credit check`, `double your money`, `make money fast`,
  `once in a lifetime`, `this is not spam`, `cash bonus`. A phrase matches ASCII
  case-insensitively, each internal space matching one or more of space, tab, CR
  or LF, with no ASCII letter, digit or `_` immediately before or after it.
- **Zero-width characters** — U+200B, U+200C, U+2060, U+FEFF. U+200D (ZWJ) is
  excluded because emoji sequences use it legitimately.

| rule id | severity | location | fires when |
|---|---|---|---|
| `spam.subject.all_caps.v1` | warning | `header {name:"subject", occurrence:0}` | the Subject has at least 8 ASCII letters and none is lowercase |
| `spam.subject.punctuation.v1` | info | `header {name:"subject", occurrence:0}` | the Subject contains `!!`, `??` or `$$` |
| `spam.subject.phrases.v1` | warning | `header {name:"subject", occurrence:0}` | the Subject contains a phrase from the list |
| `spam.from.display_name_address.v1` | warning | `header {name:"from", occurrence:0}` | the From display name contains an address-shaped token (ASCII `[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)+`) that is not ASCII-case-insensitively equal to the mailbox's own address |
| `spam.body.phrases.v1` | warning | `mime_part {path}` of the first eligible part, in wire order, that matches | an eligible text part contains a phrase from the list |
| `spam.body.zero_width.v1` | info | `mime_part {path}` of the first eligible part, in wire order, that matches | an eligible text part contains at least 3 zero-width characters |

`path` is the structural child-index path used by the HTML collector (root `[]`,
children in wire order).

Fix hints (fixed English, emitted verbatim):

- all_caps: `Write the Subject in sentence case; an all-capital subject reads as shouting and is a common spam signal.`
- punctuation: `Remove repeated !, ? or $ characters from the Subject.`
- subject phrases: `Remove high-pressure sales wording from the Subject; see the guide's phrase list.`
- display name: `Do not put an email address in the From display name unless it is the sending address; a different address there looks like spoofing.`
- body phrases: `Remove high-pressure sales wording from the text/plain body; see the guide's phrase list.`
- zero width: `Remove zero-width characters from the text/plain body; they are used to hide words from filters.`

Severity is info or warning only. Error stays the bulk checklist's
"requirement not met / not evaluated" level; a lexical heuristic must not share
it. A rule emits **at most one** finding per report — a 256-part message with a
phrase in every part yields one `spam.body.phrases.v1`, never a finding flood
that could push the report over `MAX_FINDINGS` and turn it into a failure.

### Bounds (normative)

| input | bound | source |
|---|---|---|
| raw message | 8,388,608 bytes | existing engine admission, checked before any collector |
| root header block (Subject, From) | 65,536 bytes including the separator | existing `headers()` admission |
| MIME parts / depth | 256 including root / 32 (root 0) | existing engine admission |
| aggregate decoded eligible text | **4,194,304 bytes** (new `SPAM_TEXT_BYTES`) | this story; the same figure as HTML's `HTML_BYTES` |
| spam findings | at most 6 | one per rule, by construction |

The aggregate text size is summed over **every** eligible part before any rule
matches; exceeding it fails the whole report (`AnalysisError::Failed`, REST 503
"deliverability analysis failed", MCP isError) even if an earlier part would have
matched. Truncated scanning is refused (0085: oversized analysis is a failure,
not a clean or truncated report). Work is linear: the phrase matcher is one
`regex::bytes::Regex` (a finite-automaton engine with a linear-time guarantee)
compiled once from the constant list, and the zero-width count is one pass, so
the spam collector inspects O(65,536 + 4,194,304) bytes per report beyond its
one bounded `mail-parser` parse. The cost of this bound: a message with more
than 4 MiB of decoded `text/plain`, which today gets a report, gets an analysis
failure instead.

### Signals counted elsewhere

The spam collector does not re-detect what other collectors report. The guide's
spam section names, for a reader assessing spam risk, `bulk.plain_text.v1`
(HTML-only mail), `bulk.one_click.v1` (unsubscribe), `bulk.message_id.v1`,
`bulk.date.v1`, `html.image-only.v1` and `html.tracking-pixel.v1`. 0122 records
how they would count toward a total (severity-derived weight; `not_evaluated`
findings and `demo.marker.v1` at zero). Link and URL signals belong to #258 and
DNS policy to #253; this rule set reads no URL and resolves nothing.

## Panel adjudication — 2026-11-03

Both positions agree on: a separate lazy `src/deliverability/spam.rs`; a static
typed catalogue instead of a JSON loader; one version id; a 4 MiB fail-closed
text cap with exact boundary fixtures; no load proof; weights and total proposed
in 0122 and not emitted. Adopted as above. Where they conflict:

| question | ruling | what the losing side costs |
|---|---|---|
| Version: `swarmail-spam/1` (simplicity, precedent) or family-neutral `swarmail/4` (robustness) | **`swarmail-spam/1`.** It follows `swarmail-demo/1 → bulk/1 → html/2 → html/3`. A family name also makes a parallel sibling's bump conflict *textually* on the one `RULE_SET_ID` line; two branches both writing `swarmail/4` would merge cleanly and give two rule sets one id. | The name under-describes a combined rule set; the guide's version history carries the composition. |
| Body scope: text/plain + text/html (simplicity) or text/plain only (robustness) | **text/plain only.** Phrases in raw HTML match inside attributes, CSS and comments, and phrase-splitting tags defeat them; HTML-only mail already gets `bulk.plain_text.v1`. | HTML-only spam wording is a documented false negative. |
| Body findings: one per eligible part (simplicity) or one per rule at the first matching part (robustness) | **One per rule.** Per-part findings let 256 parts × 2 rules exceed `MAX_FINDINGS` (256), so a sender could force a 503. | The report names only the first matching part. |
| Display-name severity: error (simplicity) or never error (robustness) | **warning.** Every production report already carries error-level `bulk.*.not_evaluated.v1`; error adds no predicate signal and conflates heuristic with requirement. | A spoof-shaped name ranks with a shouted subject. |
| Rule set: simplicity's four or robustness's six | **Six, mixed:** simplicity's all-caps, punctuation, display-name and phrase rules; robustness's subject/body phrase split and zero-width rule. Rejected: robustness's 60 %/70 % uppercase ratios (a "no lowercase" rule has no tunable fraction), body shouting and body punctuation counts (weak signals whose fixtures cost more than they prove), and the phrases "verify your account", "urgent" and "congratulations" (transactional mail — swarmail's main traffic — uses them). | Fewer signals; a partly shouted subject passes. |
| Matcher: literal byte search (robustness) or one `regex::bytes` alternation (simplicity) | **regex,** ASCII mode (`(?i-u)`), built once in a `LazyLock` from the constant list with `unwrap`, as `src/extract.rs` already does. Whitespace-run matching across line wraps needs more than a literal search. | One constant pattern to audit for Unicode semantics; the ASCII flag and a non-ASCII fixture cover it. |
| Weights: per-spam-rule numbers plus severity for foreign findings (robustness) or severity-derived for all (simplicity) | **Severity-derived for all** (info 1, warning 3, error 10; `not_evaluated` and demo 0), proposed in 0122: one sentence counts bulk and HTML findings with no per-id table to drift. Robustness's placement rules (weight last in each finding, total last, computed in `Collector::finish` after sort and dedup, `checked_add`, a version bump on the flip) are adopted into 0122. | Two warnings cannot be told apart by weight. |
| Guide weight column: show it (issue) or "a documented weight not emitted is a lie" (simplicity) | **Show it, labelled** "proposed in 0122 — not in the report". | One label to remove at acceptance. |
| Hard-coded `swarmail-html/3` copies: derive from `RULE_SET_ID` (robustness) or replace (simplicity) | **Replace mechanically** in live surfaces (the static OpenAPI and llms.txt cannot derive anyway); the AC6 audit stops a stale copy surviving. Historical records keep their text. | Twenty-odd one-token edits in shared files. |
| Wire fixtures' home | **New `tests/spam_rules.rs`** (robustness): siblings #253/#258 edit `tests/deliverability.rs`. | One more test binary. |

## Acceptance Criteria

Each criterion is testable; tasks.md pairs each with the test that proves it.

- **AC1 — Catalogue.** `src/deliverability/spam.rs` holds the six rules above as
  compile-time typed data (no runtime loader, no JSON file, no config knob), with
  ids, severities and fix hints exactly as listed. An inline test enumerates the
  catalogue and asserts six unique ids, each matching
  `^spam\.[a-z]+\.[a-z_]+\.v1$`; none starts with `bulk.` or `html.` or equals
  `demo.marker.v1`; severities are info or warning only; hints are nonempty.
- **AC2 — Rule semantics.** Inline unit tests drive each rule's boundary: 7 vs 8
  capital letters, and 8 capitals plus one lowercase letter; each of `!!`, `??`,
  `$$` against a single `!`; every listed phrase, a line-wrapped phrase, a phrase
  glued to a letter, digit or `_` (no match) and an upper-case phrase; a
  non-ASCII subject (no Unicode case folding); a display name equal to the
  address in another case (pass), a different address (fail), no address in the
  name (pass), a group (pass); 2 vs 3 zero-width characters, and 3 ZWJs (pass);
  an RFC 2047-encoded subject and display name that fire; a duplicate Subject or
  From where only the second occurrence would fire (no finding).
- **AC3 — Eligibility and one finding per rule.** Inline tests prove: a root
  without Content-Type is scanned; `text/html`, `text/csv`, attachment parts,
  filename/name-bearing parts and their subtrees, `message/rfc822` bodies and
  `multipart/digest` defaults are not; base64 and quoted-printable parts match
  after decoding; when parts `[0]` and `[1]` both match, the one finding's path is
  `[0]`; a 256-part message whose every leaf matches both body rules yields
  exactly one finding per body rule and a successful report.
- **AC4 — Passing and failing fixtures over the wire.** `tests/spam_rules.rs`
  sends, for each of the six rules, one failing and one passing message over real
  SMTP and reads `GET /api/v1/messages/{id}/deliverability`. The two fixtures of
  a rule differ only in its trigger. The failing report holds exactly one
  `spam.*` finding, the rule's, equal as a JSON object to the specified
  `{rule_id, severity, location, fix_hint}`; the passing report holds no `spam.*`
  finding; the two reports' non-spam findings are equal.
- **AC5 — Version.** `RULE_SET_ID` is `swarmail-spam/1`. `tests/spam_rules.rs`
  pins that literal on an ordinary report and on a demo-enabled report, through
  REST and the MCP `swarmail_get_deliverability_report` tool (text block, and
  structuredContent on a modern revision). The cache key follows through the
  existing `Inputs.rule_set_id`.
- **AC6 — No stale live identity.** On the final tree, every `git grep -n
  'swarmail-html/3'` match is historical text: decision records, release notes,
  other specs, the guide's version history, or fixture provenance prose that
  describes a past edit. `src/` (code, `html-rules.json`, `openapi.json`,
  `llms.txt`, MCP descriptors) and test assertions name `swarmail-spam/1`. The
  classification of every remaining match is recorded in the implementation
  notes.
- **AC7 — Determinism.** The same stored bytes and configuration give
  byte-identical report JSON: inline, `evaluate` twice; on the wire, two GETs of
  one message (cache hit equals miss) and the same bytes sent as two messages.
- **AC8 — Bounds.** The guide states every bound in the table above. Inline:
  eligible text of exactly 4,194,304 decoded bytes of adversarial near-miss
  content (for example repeated `act nowx `) evaluates successfully with no body
  finding; 4,194,305 bytes fails; parts of 2,097,152 and 2,097,153 bytes fail
  together; an over-cap total with a phrase in an earlier part still fails; an
  over-cap `text/plain` attachment does not count and the report succeeds; the
  existing `MAX_RAW_BYTES + 1` refusal still holds. Wire: one over-cap
  `text/plain` message in `tests/spam_rules.rs` gets REST 503 "deliverability
  analysis failed" and MCP `isError: true`, never an empty or partial report.
  No wall-clock assertion and no load test (0020 §5).
- **AC9 — Laziness and frozen guarantees.** `spam::collect` is called only from
  `deliverability::collect`; `spam.rs` uses nothing from `store`, `smtp`, `model`
  or any clock, network or async API. A wire test reads the message JSON before
  and after its first report and asserts it is byte-identical. `Store::insert`,
  `await`, pruning and ingest extraction are not edited (0001–0003, 0005).
- **AC10 — No scores.** Reports from this run carry exactly the keys
  `rule_set_id` and `findings`, and each finding exactly the four contract keys
  (asserted on a failing fixture in `tests/spam_rules.rs`). No weight or total
  code, field, cfg or feature exists in `src/`.
- **AC11 — Guide.** `docs/guide/deliverability.md` gains a "Spam signals
  (`swarmail-spam/1`)" section with one `###` heading per rule id giving what it
  matches, its severity, its proposed weight (labelled "proposed in 0122 — not in
  the report"), why, its fix hint verbatim, and a passing and a failing example;
  plus the phrase list, the bounds table, "signals counted elsewhere", known false
  negatives and positives, and a rule-set version history. An inline test
  `include_str!`s the guide and asserts, for each catalogue rule, a heading with
  its id whose section contains its severity word and exact fix hint.
  `src/static/llms.txt` mirrors the section as it mirrors the rest of the guide.
  The OpenAPI `DeliverabilityReport` description and the modern MCP report
  descriptor name the spam signals and the new id; the March descriptor keeps its
  historical text; the tests that pin these texts are updated.
- **AC12 — Existing expectations.** Apart from the identity token, no existing
  expected report in `tests/`, inline tests or the MCP fixtures changes. If one
  does, the change adds exactly the spam finding this spec requires, is listed
  with its reason in the implementation notes, and no rule is loosened to avoid it.
- **AC13 — Dependencies.** `Cargo.toml`, `Cargo.lock`, `evals/Cargo.lock` and
  `deny.toml` are unchanged; `regex` and `mail-parser` are reused.
- **AC14 — Decision record.** `docs/decisions/0122-deterministic-spam-rules.md`
  carries `- **Status:** proposed` and `docs/decisions/README.md` gains its index
  row ("proposed — awaiting the operator"). No other decision's status changes.
- **AC15 — Gates.** Every gate in `docs/house-rules.md` § Gates passes `--locked`
  on the final tree: fmt, clippy, workspace tests, the candidate binary build,
  the evals crate's fmt, clippy and tests against that binary, every recipe
  compile (reading each output for the policy-sweep warning), the coverage gate
  at exact 100 % and its selftest, the publish-guard and seal proofs, and
  `cargo deny check licenses`. `spam.rs` reaches full line, branch and function
  coverage from inline tests, with no `coverage(off)` and no `*_tests.rs` refuge
  (it has no cold path).
- **AC16 — Held, not part of this run's completion.** A per-finding `weight` and
  a report `total` as proposed in 0122 ship only after the operator accepts 0122,
  under a new rule-set id. The delivery's final report states that #260 remains
  open for this criterion.
  *2026-11-07:* 0122 was accepted on 2026-11-06, and AC16 is built by #337 as
  `swarmail-html/8`; see [`specs/finding-weight-total/spec.md`](../finding-weight-total/spec.md)
  (its AC1–AC9). The text above is unchanged.

## Out of scope

Weight and total code (AC16); configurable, localized or user-supplied phrase
lists; HTML-to-text rendering; Reply-To or organizational-domain rules (they need
a Public Suffix List — a dependency or wrong answers); URL, shortener and link
rules (#258); DNS and authentication (#253, #255); a new `Location` variant;
sharing one `mail-parser` parse with `html.rs`; load or throughput proofs.
