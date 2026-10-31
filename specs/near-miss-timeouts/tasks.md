# Ordered implementation tasks — near misses on a timed-out wait

All boxes start open. A test named here is a required witness, not a claim it
exists or passed. See [spec.md](spec.md) AC1–AC11 and [plan.md](plan.md).
Do not edit the catalogue under `evals/tests/data/`, any captured March
fixture or its provenance, or any historical specification.

- [ ] T1 — Bind the work to main `d892acb`; record the tree id and confirm
  the two captured March baselines' only timed-out lines are empty-inbox
  waits. **Proof:** the git ids in the implementation record; AC6's
  comparisons run unchanged.
- [ ] T2 — Add `src/mcp/near_miss.rs` with the three named constants, the
  char-boundary `clip`, the distinct-recipient `recipients` (To, Cc,
  envelope, capped, with the exact total) and the per-field `missed`
  projection through `Filter::matches`. **Test (inline):** a 300-byte ASCII
  subject clips to 256; a string whose 256th byte is inside a multibyte char
  clips on the boundary below; six recipients show five with `to_total: 6`;
  with `to`+`subject` supplied an entry failing only `to` reads
  `missed: ["to"]`, unsupplied fields never appear, and order is fixed (AC2,
  AC3).
- [ ] T3 — Add the bigram ranking `nearest` over candidates with
  `missed == ["subject"]`. **Test (inline):** highest overlap wins; a tie
  goes to the newest; zero score, one-character filter and no subject filter
  give `null`; a closer-subject candidate that also failed `to` is never
  chosen; a CJK subject ranks (AC4).
- [ ] T4 — Give `List` in `src/mcp/focused.rs` a `with_cap` constructor
  (default unchanged) and assemble `near_misses` from one
  `Store::list(inbox, &Filter::default())` snapshot on `Store::offload`:
  `since_ms` candidacy, report-wait skip of matching mail, focused no-skip,
  `last_arrival_ms` from acceptance order, `None` on an empty snapshot.
  **Test (inline, fresh `Store::new`):** eleven candidates show ten with
  `arrivals_total: 11` and `arrivals_truncated: true`; `since_ms` excludes
  older mail from `arrivals`/`arrivals_total` while `last_arrival_ms` still
  reports it; out-of-order `received_ms` pins acceptance order; an empty and
  a removed inbox return `None` (AC2, AC3, AC5, AC6).
- [ ] T5 — Attach the object in the `WaitOutcome::Timeout` arm of
  `swarmail_wait_for_email` and in the focused `None` arm, before
  `text_result`, leaving the found arm, `total`, the error arms and both wait
  owners untouched. **Test (wire, `tests/mcp.rs`, `tests/focused_waits.rs`):**
  exact populated object on each of the three tools after real SMTP
  delivery; focused `missed: []` on filter-matching mail without a code /
  with no selected link; a mistyped `anchor_text` selector still lists the
  mail; found answers byte-identical; inbox contents and count unchanged
  after a timed-out wait; `git diff --stat` shows no `src/store.rs` change
  (AC1, AC2, AC8, AC11).
- [ ] T6 — Prove the tenant boundary. **Test (wire, `tests/mcp.rs`):**
  deliver through SMTP AUTH to a sibling inbox while the waiter's wait is
  parked (sequenced on the readiness gauge, not a sleep); the waiter's
  timed-out answer contains none of the sibling id, subject or address, and
  does contain the waiter's own non-matching mail (AC9).
- [ ] T7 — Add the modern schema variants: `near_miss::schema` for the closed
  object and entry, the third `oneOf` arm for `swarmail_wait_for_email`, the
  second `timed_out` arm for each focused wait. **Test
  (`tests/mcp_revisions/structured.rs`):** every populated and empty timed-out
  answer validates on June and November over HTTP and stdio; March text equals
  the modern `structuredContent`; negatives (extra member, missing member,
  unknown `missed` value, non-null non-object `nearest_subject`, `near_misses`
  on a found answer) are refused; the captured March comparisons pass
  unchanged (AC6, AC7).
- [ ] T8 — Update the exact-shape cases in `tests/mcp_revisions/focused.rs`
  whose `timed_out` answers sit over populated inboxes (`links` with
  `path: ^/nowhere$`, `history` with `anchor_text: ""`) to the full object;
  keep the `empty` inbox case unchanged. **Test:** the file's own cases
  (AC7).
- [ ] T9 — Document the member in `README.md` and `src/static/llms.txt`:
  presence rule, caps, `missed`, `nearest_subject` as a hint not a selection,
  absent key means the inbox holds nothing. **Test:**
  `tests/listener_contract_docs.rs` served-surface checks stay green; a
  stale-claim assertion that the timeout carries only `matched`, `total`,
  `timed_out` is replaced with the new sentence (AC1).
- [ ] T10 — Add the three helper modes to `evals/tests/common/mod.rs`
  (`near_miss_sibling`, `near_miss_sibling_nearest`, `near_miss_slow`) and the
  proof test `near_miss_controls_recover_after_a_timed_out_wait` in
  `evals/tests/proof.rs`; update `evals/README.md`'s control count and add the
  honest limit that the live recovery rate is not measured. **Test:** sibling
  recovery passes with no trap event; the nearest-subject control loses as
  `wrong_message` with a `foreign_token` event; slow-delivery's first answer
  has `total: 0` and no `near_misses`, then release and confirmation pass; no
  file under `evals/tests/data/` changes (AC10).
- [ ] T11 — Run every house gate locked, including the evals crate, the
  coverage gate with its selftest, publish-guard and seal proofs,
  `brokkr compile --bundle .brokkr/recipes/self` and `cargo deny check
  licenses`; record the receipts and the exact coverage figures in the
  implementation record, naming the CI-owned contexts as owed. **Proof:** the
  receipts; `missed == 0` lines, branches and functions (AC11).
- [ ] T12 — Commit unsigned in the repository's style; never push, merge,
  publish or accept 0106. **Proof:** `git log --format='%G? %h'` shows the
  branch's new commits unsigned and the operator seals before merge (0056).
