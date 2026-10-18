# Ordered implementation tasks: extraction fidelity

The design is drafted; unchecked tasks are required implementation work.
Acceptance IDs refer to [spec.md](spec.md). Keep
[0040](../../docs/decisions/0040-extraction-fidelity.md) proposed until the
operator accepts it. Every task below includes its proving check.

- [ ] **T01 — Establish the regression baseline.** On a feature branch, add
  real SMTP/REST reproductions for mixed MIME, adjacent table-cell codes and
  encoded URL delimiters. **Proof:** at least one behavioral red per #74,
  #93 and #94 against unchanged extraction; record SHA, commands and exact
  failed assertions, not compilation failures (AC-delivery).
- [ ] **T02 — Select actual body candidates.** Replace candidate-zero checks
  with first matching actual types in parser order; retain outer-message
  ownership and absence. **Proof:** inline MIME cases and SMTP/REST tests for
  both orders, alternatives, multiple/empty genuine bodies, nested multipart,
  attachments and attached HTML message/rfc822 pass (AC74).
- [ ] **T03 — Pin complete named mapping data.** Add the private sorted table
  from a pinned authoritative snapshot, license/attribution and deterministic
  regeneration instructions; add no dependency or runtime fetch. **Proof:**
  verify all 2,125 semicolon-required mappings against that snapshot, sorted
  uniqueness, longest name within the window and maximum 6/5 byte expansion;
  `cargo deny check licenses` passes (AC94-edges).
- [ ] **T04 — Extend the existing bounded decoder.** Support all replacement
  scalars with the existing window/stats owner; keep numeric failure policy
  and decode-once semantics. **Proof:** full-table and numeric/malformed
  unit tests, multi-scalar/growing examples, and both unchanged exact
  500,000-scan byte pins pass (AC94-edges, AC-bounds).
- [ ] **T05 — Replace extraction's renderer with the tokenizer.** Implement
  forward text runs, tag/attribute states, suppression nesting, raw closes,
  comments and EOF policy, then use the ASCII code regex per decoded run.
  **Proof:** all ten #93 inputs over SMTP/REST match the exact spec arrays;
  inline boundary/Unicode/quote/EOF/entity/preference/dedup cases pass
  (AC93, AC93-edges).
- [ ] **T06 — Prove tokenizer work and slicing.** Extend instance-local stats
  without changing existing fields/seams; document monotonic progress and
  fixed-width probes. **Proof:** exact counts for repeated tags, bare `<`,
  suppressed nesting and raw-text near misses, deterministic length-at-most-
  five enumeration, and explicit multibyte fixtures pass (AC-bounds).
- [ ] **T07 — Decode before cutting HTML links.** Decode HTML once before URL
  matching, enable ASCII case-insensitive HTTP(S) schemes and retain source
  spelling, delimiters, punctuation and stable deduplication. **Proof:** all
  seven #94 SMTP/REST rows, encoded scheme, multi-URL delimiter, plain-literal
  entity and decode-once cases pass (AC94, AC94-edges).
- [ ] **T08 — Prove raw preservation and historical scope.** Extend existing
  wire and persistence suites without changing persistence production code.
  **Proof:** mixed MIME `/raw` byte equality, POP3 RETR normalization/
  transparency, verbatim legacy restore and differing fresh ingest of the
  same raw mail pass (AC74-raw, AC-restore).
- [ ] **T09 — Align public documentation and ruling references.** Append the
  dated 0031 clarification, index proposed 0040, add dated release notes, and
  replace obsolete extractor renderer/no-growth comments. **Proof:** review
  each contract against AC74/93/94 and the restore test; local links resolve,
  original accepted ruling text remains, and no text promises automatic
  historical repair or browser equivalence (AC-restore, AC-delivery).
- [ ] **T10 — Verify and hand off.** Run formatter before format check, locked
  workspace clippy/tests, bundle compile, exact coverage without TMPDIR
  override and license check using the plan's commands. **Proof:** six clean
  gate results, exact 100/100/100, repaired regression results and an inspected
  diff with no weakened scan pins, exclusions or allowlist changes. Record
  environmental blockers honestly; implementation is not complete while
  required work remains (AC-delivery).
