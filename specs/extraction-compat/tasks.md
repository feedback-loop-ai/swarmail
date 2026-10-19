# Ordered tasks — extraction and compat

These are implementation obligations, not a completion receipt. Proposed
test names may be adjusted to local naming; preserve their issue/AC mapping.
Each changed behavior needs a red against reviewed behavior and a green
after repair. Existing passing checks are preservation evidence.

**Historical first-run status (2026-10-18, implement seat at d350d5b).** T01 and T04–T09
are done with red/green evidence (the reds observed on `067ab91`, whose
behavior on these paths is `2a325c9`'s); T10 is done apart from the
entity-table reconciliation, and T11 is green apart from the bundle compile,
which the verify seat runs. **T02 is not done:** no WHATWG `entities.json`
snapshot was available offline and this seat holds no fetch capability, so
the 71-name interim table stands, unclaimed as complete. T03 is done except
its two named-entity rows (`caf&eacute;`, `go&rarr;`), which wait on T02.

**Resume confirmation (2026-10-18, chief).** Operator commit `2987ee2`
supplies the exact snapshot; design verified its SHA-256 and 2,231/2,125/106
counts. The source-acquisition blocker above is historical. T02 is split
into T02a–T02e below and remains incomplete until all five proofs land. Continue on
`epic120-extraction-compat`; preserve all three existing signed commits.
T03 has only the two named-entity rows left; T10 has only their provenance
and status reconciliation left. Rerun T11 on the completed table; prior
green gates cannot establish coverage of new data/tests. T12 is the new
signed delivery receipt. No design checkbox claims implementation success.

**Resume implementation status (2026-10-18, implement seat).** T02b–T02e,
T03 and T10 are done; T11 is green on the final tree apart from the bundle
compile (brokkr is refused in this sandbox; the verify seat runs it).
Reds observed before the table swap: `all_whatwg_entities_match_snapshot`
(whole-slice inequality on the 71-name table), `legacy_names_need_their_semicolon`
(`&AElig;x` left literal) and `smtp_issue_136_link_fidelity`
(`…/caf&eacute`, `…/go&rarr`); a same-count `zopf` U+1D56B→U+1D56C mutation
failed the snapshot test while both table-self tests stayed green.
The formatted `NAMED` block is byte-identical to a one-off Rust derivation
using the generator's exact filter, sort and escape rules, and the
packaged crate's `extract` tests pass offline from `target/package`.
**T02a stays open on one point:** the hardened generator is written but
this seat's sandbox refused every `python3` invocation, so its digest
refusal and byte-identical output are unexecuted. Proof owed:
`python3 -I scripts/gen-entities.py scripts/data/html-entities.json` diffed
against lines 39–2165 of `src/extract/entities.rs`, and the same run over a
one-byte-altered copy exiting non-zero with empty stdout.

- [x] T01 Establish `epic120-extraction-compat` from fetched main at or after
  `067ab91`; preserve previous batches and capture baseline reds. Proof:
  ancestor/branch/signature checks and the T03–T09 regression receipts
  mapped to #136–#139; no claim based solely on baseline compilation.
- [ ] T02a (#136, AC2) Complete the existing generator's pinned-input
  checks using standard-library SHA-256, ASCII/unique-name/count/scalar
  validation and source-shape checks; do not replace or fetch the supplied
  JSON. Proof: the supplied digest passes; a one-byte-altered temporary
  input fails before emitting a table; valid generation produces exactly
  2,125 byte-sorted mappings with all scalars preserved.
- [x] T02b (#136, AC2) Replace the interim NAMED block with generated data;
  record digest, WHATWG title/author/source/date, CC-BY-4.0 URL, exact command
  and the snapshot-to-table transformation inline. Proof: regeneration
  matches the formatted committed block, direct SHA-256 matches the spec,
  and provenance review confirms attribution without an allowlist change.
- [x] T02c (#136, AC2) Add inline measured
  `all_whatwg_entities_match_snapshot`, independently deriving every value
  from JSON codepoints and checking characters, whole-slice equality,
  lookup and decoding. Pin all counts, counterpart equality, strict order,
  exact case, scalar sequences, window/expansion and no ASCII-digit values.
  Proof: all mappings pass; the interim 71-entry table and an isolated
  same-cardinality wrong-scalar mutation fail. No Python in Rust tests.
- [x] T02d (#136, AC2/AC3 preservation) Prove all 106 legacy aliases stay
  literal as `&n` and `&n=1`, while `&n;x` decodes the complete named
  mapping then `x`; retain `&amp`, `&notit;`, `&amp=1` controls. Proof:
  inline decoder matrix plus unchanged exact reference work/count tests.
- [x] T02e (#136, AC2) Include only the existing JSON fixture in Cargo.toml's
  package file list. Proof: offline locked package listing contains that
  exact path; the independent entity test compiles and passes from isolated
  packaged sources without Python, network or a second snapshot copy in git.
- [x] T03 (#136, AC1–AC3) Add only the remaining `caf&eacute;` attribute
  and `go&rarr;` HTML-text rows to the real SMTP/REST link fixture; expect
  exactly `https://x.io/café` and `https://x.io/go→` in array order. Proof:
  `smtp_issue_136_link_fidelity` fails those rows with the interim table and
  passes with the full table. Preserve already implemented quote boundaries,
  punctuation balancing and numeric guards via
  `attribute_links_keep_encoded_quotes`, `links_keep_balanced_url_punctuation`,
  `numeric_references_refuse_signs`, real SMTP/REST `smtp_issue_136_link_fidelity`,
  and deterministic region/trim work tests; preserve `decoded_delimiters_end_the_link`,
  `an_ampersand_run_decodes_in_bounded_work`,
  `the_reference_window_counts_characters_not_bytes` and all tokenizer pins.
- [x] T04 (#139, AC9–AC10) Introduce one bounded 64-distinct-code collector
  across runs, without changing scan accounting. Proof:
  `codes_cap_bounds_collection_before_allocation` covers 0/64/65, duplicates,
  ten-digit maximum, HTML runs and at most 64 retained values; collect-then-
  truncate is an explicitly failing negative control.
- [x] T05 (#137, AC4; #139, AC9) Retain chosen MIME identities and use direct
  sibling alternative preference, otherwise source-order collection through
  the same cap. Proof: real SMTP `mixed_footers_do_not_hide_html_codes` and
  `only_direct_alternative_siblings_prefer_text`, exact arrays from both
  issue fixtures, reversed order/nested related/empty-text-code fallback,
  mixed-source 65th code; all #74/alternative/body/raw/attachment pins stay.
- [x] T06 (#138, AC5/AC7) Extend the finite refusal detector on both compat
  containing searches, keeping accepted literals untouched. Proof:
  `wrapped_field_prefixes_refuse_by_name` with negative/quoted/parenthesized/
  whitespace cases and URL/plain-word/explicit-kind positive controls;
  `latest_timezone_is_a_documented_refusal` pins tz's named 400 and existing
  empty/filter behavior.
- [x] T07 (#138, AC6) Sort all scoped compat candidate views by the existing
  descending timestamp/ID order and correct the latest comment. Proof:
  `compat_views_share_timestamp_order` builds deterministic reverse-acceptance
  and equal-stamp fixtures, checks real scoped/global list/search pages and
  latest, then reopens persistence; existing native/POP3/pruning pins remain.
- [x] T08 (#139, AC8) Move complete single-message JSON/plain/raw/header and
  download response builds, including latest and native aliases, into
  awaited existing offload. Proof:
  `held_single_message_builds_leave_smtp_and_health_serving` extends the
  current real-server harness with construction-site gates, HTML-only render,
  native serialization, inline negative control, completed bytes/content
  types and unchanged 404/raw fidelity. Retain the existing four page proofs.
- [x] T09 (#139, AC9–AC10) Prove the cap on SMTP and seed ingest, persisted
  records and real webhook payloads; preserve historical rows explicitly.
  Proof: `numeric_mail_has_bounded_codes_and_response_bytes` uses exactly the
  8 MiB/1,048,576-number fixture, 641-byte seven-digit array, <9 MiB full
  native list, <16 KiB Mailpit summary and <40 MiB complete MailHog page;
  `fresh_codes_stay_capped_after_restart_and_webhook` and
  `historical_codes_restore_verbatim` distinguish the two contracts. Existing
  exact MailHog 64 MiB page-budget tests remain green.
- [x] T10 (AC7/AC11) Reconcile only outstanding entity provenance/status
  in docs, release notes, the old extraction-fidelity task and decision
  pointers after T02/T03 proof. Preserve the already published contracts,
  dated blocker history, historical-row residual and proposed 0040/0049
  statuses. Proof: served-contract tests agree with real HTTP query,
  ordering and cap fixtures; review checks provenance, explicit historical/
  link/pool residuals and no invented acceptance of 0049.
- [ ] T11 (AC11) Run the complete house inventory in plan.md, formatting
  before each check, exact coverage without TMPDIR override, both script
  selftests, licenses and fail-closed native-tls proof; verify runs pinned
  brokkr compile. Proof: successful final-tree receipts for every required
  gate; refused cargo or missing provenance leaves work incomplete.
- [ ] T12 (AC11) Record issue-to-regression red/green evidence in the signed
  implementation commit; leave the authorized branch clean for local handoff.
  Proof: `git verify-commit HEAD`, branch/status and staged-path inspection;
  no unsigned commit, push, PR, merge or issue closure. Artifact validation
  and signed delivery are recorded separately from implementation gates.
