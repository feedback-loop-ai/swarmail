# Tasks — built-in deterministic spam signals (#260)

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md). Each task names the test
that proves it. Inline tests live in `#[cfg(test)] mod tests` of
`src/deliverability/spam.rs`; wire tests in `tests/spam_rules.rs`.

- [ ] **T1 — Audit before code (AC12).** On the base tree, list every Subject,
  From and text/plain body used by an expected report in `tests/`, inline tests
  and `tests/fixtures/mcp-structured-output/`, and check each against the six
  rules. Record the result in `specs/spam-signal-rules/implementation.md`.
  *Proof:* the recorded audit; after T7, `cargo test --workspace --locked` passes
  with only identity-token edits to existing expectations (or the AC12 additions
  listed with reasons).
- [ ] **T2 — Catalogue (AC1).** Add `src/deliverability/spam.rs` with the
  `Signal` enum, `SIGNALS`, exhaustive `id`/`severity`/`hint`, the ids,
  severities and hints verbatim from the spec. *Proof:* inline
  `catalogue_is_closed_and_disjoint` — six unique ids matching
  `^spam\.[a-z]+\.[a-z_]+\.v1$`, none `bulk.`/`html.`/`demo.marker.v1`, info or
  warning only, nonempty hints.
- [ ] **T3 — Subject and From rules (AC2).** First root Subject / From in wire
  order from the parsed header list; all-caps, punctuation, subject-phrase and
  display-name predicates. *Proof:* inline `subject_rules_boundaries` (7 vs 8
  capitals, 8 capitals + 1 lowercase, `!!`/`??`/`$$` vs `!`, non-ASCII subject,
  RFC 2047 subject that fires), `display_name_rule_boundaries` (same address in
  other case passes, different address fails, plain name passes, group passes,
  encoded display name fires), `first_occurrence_only` (duplicate Subject and
  duplicate From where only the second would fire: no finding; missing Subject
  and missing From: no finding).
- [ ] **T4 — Phrase matcher (AC2).** `LazyLock<regex::bytes::Regex>` built from
  the 14-entry constant list: `(?i-u)\b(?:…)\b`, spaces → `[ \t\r\n]+`, others
  `regex::escape`d. *Proof:* inline `phrases_match_exactly` — every phrase, its
  upper-case form and a CRLF-wrapped form match; each glued to a letter, digit or
  `_` on either side does not; `no credit chec\u{212A}` and
  `thi\u{17F} is not spam` do not; "verify your account", "urgent",
  "congratulations" do not.
- [ ] **T5 — Body walk, eligibility and one finding per rule (AC2, AC3).** Walk as
  `html.rs` does; eligible = `text/plain` or no Content-Type, not under an
  attachment/filename/name, not inside `message/*`; encoding problem or
  undecodable part → `Failed`. Zero-width count over U+200B, U+200C, U+2060,
  U+FEFF. *Proof:* inline `body_eligibility` (no-Content-Type root scanned;
  `text/html`, `text/csv`, attachment, `name=`/`filename=` subtree,
  `message/rfc822`, digest default not scanned; base64 and quoted-printable
  decoded), `zero_width_boundaries` (2 vs 3, 3 ZWJs pass), `first_part_wins`
  (`[0]` and `[1]` match → one finding at `[0]`), `many_parts_one_finding` (256
  matching leaves → one finding per body rule, report succeeds), and
  `parse_failure_fails` (`spam::collect(b" ", …)` → `Failed`).
- [ ] **T6 — Text cap (AC8).** `SPAM_TEXT_BYTES = 4_194_304`, summed over every
  eligible part before matching. *Proof:* inline `text_cap_is_exact` — exactly
  4,194,304 bytes of repeated `act nowx ` evaluates with no body finding;
  4,194,305 fails; parts of 2,097,152 + 2,097,153 fail; an over-cap total with a
  phrase in part `[0]` fails; an over-cap `text/plain` attachment is ignored and
  succeeds; `evaluate` of `MAX_RAW_BYTES + 1` still fails.
- [ ] **T7 — Wire in and bump (AC5, AC7, AC9).** `mod spam;`,
  `spam::collect(raw, out)?;` after `html::collect`, `RULE_SET_ID =
  "swarmail-spam/1"`, `html-rules.json` id. *Proof:* inline
  `evaluate_is_deterministic` (same bytes twice → equal serialized JSON) and the
  updated existing inline report literals; `rg -n 'spam::collect' src` shows the
  single call site; `spam.rs` has no `use` of `store`, `smtp`, `model`, `tokio`,
  `std::time` or `std::net` (reviewed in the diff).
- [ ] **T8 — Live identity (AC6).** Replace `swarmail-html/3` with
  `swarmail-spam/1` in `src/`, `src/static/*`, MCP descriptors and test
  assertions/fixtures (one-token edits with a dated provenance note in the MCP
  fixture README/provenance). *Proof:* `git grep -n 'swarmail-html/3'` — every
  remaining match classified as historical in `implementation.md`; the existing
  suites `tests/deliverability.rs`, `tests/bulk_sender.rs`,
  `tests/mcp_revisions` and `html.rs` inline tests pass.
- [ ] **T9 — Wire fixtures, one pair per rule (AC4).** `tests/spam_rules.rs`:
  table of `(rule, failing raw, passing raw, expected finding JSON)` for all six
  rules, each pair differing only in its trigger, sent over real SMTP into a
  per-test inbox. *Proof:* `cargo test --locked --test spam_rules
  every_rule_has_a_failing_and_passing_fixture` — failing report has exactly that
  one `spam.*` finding as an exact JSON object; passing report has none; non-spam
  findings equal across the pair.
- [ ] **T10 — Version pin and no scores (AC5, AC10).** *Proof:*
  `cargo test --locked --test spam_rules report_identity_and_shape` —
  `rule_set_id == "swarmail-spam/1"` on an ordinary and a demo-enabled report,
  via REST, the MCP tool's text block and a modern revision's structuredContent;
  report keys exactly `{rule_set_id, findings}`, finding keys exactly the four.
- [ ] **T11 — Determinism and laziness on the wire (AC7, AC9).** *Proof:*
  `cargo test --locked --test spam_rules spam_report_is_deterministic_and_lazy` —
  two GETs byte-identical; the same bytes as two messages give equal reports; the
  message JSON read before the first report GET equals the read after it.
- [ ] **T12 — Oversized text fails over the wire (AC8).** *Proof:*
  `cargo test --locked --test spam_rules oversized_text_fails_closed` — a
  4,194,305-byte `text/plain` message is accepted by SMTP (250), REST returns 503
  "deliverability analysis failed", MCP returns `isError: true`.
- [ ] **T13 — Guide and served docs (AC11).** Add the spam section to
  `docs/guide/deliverability.md` (per-rule `###` with matches, severity, proposed
  weight labelled "proposed in 0122 — not in the report", why, verbatim hint,
  passing and failing example; phrase list; bounds table; signals counted
  elsewhere; false negatives/positives; version history), mirror it in
  `src/static/llms.txt`, update the OpenAPI `DeliverabilityReport` description
  and `MODERN_REPORT_DESCRIPTION`. *Proof:* inline `guide_documents_every_rule`
  (`include_str!("../../docs/guide/deliverability.md")`: per id a heading whose
  section holds the severity word and exact hint); the existing served-docs and
  llms.txt tests in `tests/api.rs` and `tests/deliverability.rs`; the descriptor
  clause list in `tests/mcp_revisions/structured.rs` gains "spam signals" and
  the new id.
- [ ] **T14 — Decision index (AC14).** Add the 0122 row to
  `docs/decisions/README.md` as "proposed — awaiting the operator". *Proof:*
  `git diff main -- docs/decisions/` shows only the new 0122 file and that row;
  no other status changed.
- [ ] **T15 — Implementation ledger (AC6, AC12, AC16).** Write
  `specs/spam-signal-rules/implementation.md`: T1 audit, AC6 grep classification,
  any AC12 additions, gate receipts, and the statement that #260 stays open for
  the held weight/total (AC16). *Proof:* the file exists and names each item.
- [ ] **T16 — Gates (AC13, AC15).** Run every house gate `--locked` on the final
  tree: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets
  --locked -- -D warnings`; `cargo test --workspace --locked`; `cargo build
  --locked --bin swarmail`; the evals crate's fmt, clippy and
  `SWARMAIL_BIN=<candidate> cargo test --locked`; every recipe compile (reading
  output for the policy-sweep warning); `bash scripts/coverage-gate.sh` (exact
  100 %) and `bash scripts/coverage-gate.sh selftest`; `bash
  scripts/publish-guard-test.sh`; `bash scripts/seal-branch-test.sh`; `cargo deny
  check licenses`. *Proof:* every command exits 0; `git diff main --
  Cargo.toml Cargo.lock evals/Cargo.lock deny.toml` is empty. No load is applied.
