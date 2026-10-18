# Ordered tasks — extraction and compat

These are implementation obligations, not a completion receipt. Proposed
test names may be adjusted to local naming; preserve their issue/AC mapping.
Each changed behavior needs a red against reviewed behavior and a green
after repair. Existing passing checks are preservation evidence.

**Implementation status (2026-10-18, implement seat).** T01 and T04–T09
are done with red/green evidence (the reds observed on `067ab91`, whose
behavior on these paths is `2a325c9`'s); T10 is done apart from the
entity-table reconciliation, and T11 is green apart from the bundle compile,
which the verify seat runs. **T02 is not done:** no WHATWG `entities.json`
snapshot was available offline and this seat holds no fetch capability, so
the 71-name interim table stands, unclaimed as complete. T03 is done except
its two named-entity rows (`caf&eacute;`, `go&rarr;`), which wait on T02.

- [x] T01 Establish `epic120-extraction-compat` from fetched main at or after
  `067ab91`; preserve previous batches and capture baseline reds. Proof:
  ancestor/branch/signature checks and the T03–T09 regression receipts
  mapped to #136–#139; no claim based solely on baseline compilation.
- [ ] T02 (#136, AC2) Pin the WHATWG JSON, actual SHA-256, attribution/license
  and generation command; regenerate all 2,125 mappings. Proof:
  `all_whatwg_entities_match_snapshot` compares every mapping to the JSON,
  validates sorted unique names/scalars/expansion and reproducible generation;
  a changed mapping with unchanged count fails the proof.
- [ ] T03 (#136, AC1–AC3) Share source quote boundaries with the link consumer,
  implement linear punctuation balancing and ASCII numeric guards. Proof:
  `attribute_links_keep_encoded_quotes`, `links_keep_balanced_url_punctuation`,
  `numeric_references_refuse_signs`, real SMTP/REST `issue_136_link_fidelity`,
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
- [ ] T10 (AC7/AC11) Publish contracts in README, served docs/schema and
  release notes; index 0049 as proposed and append dated pointers without
  rewriting accepted rulings. Reconcile old full-table task claims with
  actual evidence. Proof: served-contract tests agree with real HTTP query,
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
