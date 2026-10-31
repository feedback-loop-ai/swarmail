# Ordered implementation tasks — focused mail waits

All boxes start open. A test named here is a required future witness, not a claim
it exists or passed. See [spec.md](spec.md) AC1–AC10 and [plan.md](plan.md).
Do not edit historical specifications or infer acceptance. Implementation may
record actual results separately; the engine owns verify/review verdicts.

- [ ] T1 — Bind work to provisional 25b367d/tree 91c8ada, verify issue packet and
  historical patch digests, freeze existing March fixtures, inspect current main,
  reviewed prerequisite availability and decision allocations. **Proof:** exact
  git identities/digests, unchanged fixture hashes, reconciliation ledger showing
  the 0092 #155/OPTIONS collision, allocated #156/0093 and provisional 0094; missing
  final reviewed heads remain explicitly owed (AC9/AC10).
- [ ] T2 — Add closed internal occurrence/source types and skipped optional Email
  metadata; update constructors mechanically. **Test:** inline serialization
  proves old Email bytes for None, Some([]), populated metadata; required-nullable,
  extra-field/type/source negatives prove strict new-column shape (AC1/AC2/AC7).
- [ ] T3 — Extend current source-aware tokenizer with occurrence ownership and
  lexical anchor/rel collection; derive legacy URL projection from the same
  result. **Test:** exact ordered arrays for both quoted-attribute orders,
  unquoted href before rel/title, adjacent and repeated destinations, quoted
  neighbour URL, duplicate/empty attributes, nested/block/br labels, one-decode
  Unicode/punctuation, malformed/suppressed/conditional sources; retain exact
  work counters and source-proportional association/label allocation (AC1).
- [ ] T4 — Record metadata in shared SMTP/seed build_email before insert.
  **Test:** real SMTP and seed readback through focused link results for labelled
  anchors/rel/origin; MIME body/attachment/direct-alternative controls retain exact
  codes/links/raw/bodies and normal Email output (AC1/AC7/AC9).
- [ ] T5 — Migrate nullable link_metadata_json and persist it in the existing
  insert/prune/counter transaction, with strict non-NULL restore. **Test:** genuine
  old schema, None/[]/populated/corrupt/missing-member rows, exact historical fields
  through two restarts, and existing controlled commit refusal under cap pressure
  leaves no phantom state or wake (AC2/AC9).
- [ ] T6 — Implement one narrow focused lifecycle beside unchanged report waits.
  **Test:** deterministic owner-local subscribe-window/Closed/lag/unusable-flood
  interleavings; paused ineligible scan crossing expiry after eligible commit
  requires fresh terminal capture; advancing time during terminal scan still
  stops once; zero timeout scans once. Use a declared cold-path harness only where
  0020 requires it. Run legacy/count-zero/report-ready/cold/failure/deadline
  regressions independently (AC5/AC9).
- [ ] T7 — Build newest-code and complete-snapshot link evaluators with bounded
  presentation. **Test:** reversed timestamp/UUID versus acceptance order, restart
  seq order, per-email duplicates, repeated older codes, multi-code newest without
  fallback, overflow-safe/future ages, stored sender including envelope fallback,
  historical unknown provenance; repeated URL versus distinct URL after entry 250,
  all totals/truncation and large historical code arrays (AC3/AC4).
- [ ] T8 — Add strict focused MCP admission and bounded selectors before parking.
  **Test:** omission/defaults, explicit null/types/unknown keys, timeout 0/60000/
  excess, regex 1024-byte/over-byte/compiled-size/invalid cases; same-occurrence
  AND, empty/unknown anchor, parsed host/IDNA/IPv6/path/escaped percent/query,
  exact-host decoy controls and no subscription on refusal (AC4/AC5).
- [ ] T9 — Wire adapter shutdown/readiness and preserve operational errors.
  **Test:** stop only after acknowledged focused park, then require error and
  gauge release; pre-stopped control, lag/Closed recovery and retained report
  behavior. Never use an immediate !is_finished assertion or 1 ms scheduling
  premise as synchronization (AC5/AC9).
- [ ] T10 — Prove each to/from/subject/since_ms filter independently on BOTH new
  tools. **Test:** real SMTP/HTTP positive and negative cases whose selected IDs
  change if that one filter is removed; include cc/envelope recipient and inclusive
  timestamp controls. Combine owner-acknowledged post-park foreign/unusable delivery
  with terminal wire outcomes and target eligible delivery; no sleeps (AC6).
- [ ] T11 — Add two appended descriptors, read-only annotations and finite closed
  output schemas; preserve one-value formatting and report fragments. **Test:**
  exact fifteen-name descriptor/schema parity, independently expected variants,
  required/null/omission/range/250 caps and selected-on-ambiguity negatives; retain
  evaluator unknown-assertion/all-branch controls and OpenAPI source pins (AC7).
- [ ] T12 — Extend real HTTP and binary stdio workflows for all fifteen tools
  on June AND November, plus focused history/error cases. **Test:** each success
  validates against its descriptor and matches its JSON text; all selected/
  ambiguous/timed_out forms, independent payloads, errors/notifications/mixed
  exchanges, all report forms and existing transport boundaries. Preserve both
  existing March captures; compare original descriptor subsequence and existing
  result bytes, including fresh metadata, plus separate new-tool expectations.
  Retain actual-byte 10 MiB refusal correlation and continuation (AC7/AC9).
- [ ] T13 — Add application consumers for stale-code and look-alike-link traps
  using returned values. **Test:** local fixture application succeeds with selected
  values and independently fails stale/decoy actions, including since_ms freshness
  and ambiguity without guessing. Run fixture suite only with authentic required
  coordination/executable provenance; keep #201 standalone proof unchecked while
  unavailable. No paid trials or manufactured eval receipt (AC8/AC10).
- [ ] T14 — Update current README/llms examples and index 0094 as proposed after
  collision recheck. **Test:** documented calls match real schemas/results; exact
  host patterns, age/sender/history/timeout/caps and legacy behavior agree with
  producer tests; historical specs/decisions/captures remain unchanged. Reconcile
  eventual reviewed #153/#152 OPTIONS/current main and rerun affected tests before
  final delivery; do not silently satisfy this with provisional heads (AC9).
- [ ] T15 — Commit implementation unsigned and run the full house inventory plus
  pinned design/delivery proofs with verified shims and engine digest. **Test:**
  locked fmt/clippy/tests, compile without sweep warnings, exact zero-miss coverage
  plus selftest, publish/seal proofs, licenses and fail-closed native-tls absence;
  retain statuses/log digests/commit/tree including any failure. Required CI,
  actual-client, #151 baseline, #201 coordination and operator seal proofs remain
  separately attributed. Engine verify and five protected reviewers must provide
  fresh results with the inherited HIGH review-integrity residual unchanged
  (AC10). No push/merge/publication/tag/closure/acceptance.
