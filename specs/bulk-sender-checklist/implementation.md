# Bulk-sender checklist implementation — 2026-10-26

The lazy report now uses `swarmail-bulk/1` and evaluates the seven checks in
proposed 0089. Its serialized Report/Finding shapes and adapters remain the
0085 contract. Fixed hints cite the supplied Gmail, Yahoo and RFC references.
The demo marker and its independent flag remain available.

`src/deliverability.rs` owns original-byte singleton header inspection, current
Message-ID/Date syntax, complete one-click URI lists and an iterative outer MIME
metadata walk. Inspection finishes the complete bounded tree, including attached
containers, before returning a report. It does not decode body transfers, extract
links/codes, consult a clock, resolve names or contact unsubscribe endpoints.
Date calendar/weekday arithmetic reduces years modulo the Gregorian 400-year
cycle after enforcing the year floor; long numeric years do not overflow or
inherit chrono's artificial year ceiling.
The raw/header/aggregate/entity/depth limits are inclusive analysis limits;
accepted mail exceeding them remains stored and available through `/raw`.

Authentication inputs are immutable three-state observations with policy and
observation identities included in cache equality. Production supplies Unknown
for every fact. The per-Service test seam is a OnceLock: the trusted snapshot
cannot change during a request, scan or wait. #253/#255 retain verification,
provenance, mail binding, alignment policy, expiry/refresh and wake ownership.
A Weak selected-mail identity remains a separate mandatory cache key component.
Sender Authentication-Results and DKIM-Signature headers cannot supply evidence.

Proof mapping:

- T1/T6: measured `bulk_auth_truth_tables` exhausts all fact combinations against
  an independent possibility-set oracle. `bulk_auth_cache_identity` changes
  every fact and identity; existing tests cover config/version and Weak ownership.
  Collector capacity and byte limits retain refusal proofs.
- T2/T3: measured `bulk_header_grammar` exercises current syntax and rejection
  edges. Real SMTP/report `bulk_one_click_identity_date_pairs` pins complete
  hints, IDs, severity and null/0/1 coordinates for passes/failures/duplicates.
- T4/T5: `bulk_plain_text_pairs_preserve_stored_fields` proves genuine body
  presence, attachment ancestry, embedded/digest exclusion and cold/warm byte
  and stored-field identity. `bulk_analysis_budget_boundaries_and_incomplete_mime`
  proves every inclusive limit and one-over case through SMTP/report/raw reads.
  Measured `bulk_mime_metadata_grammar` covers malformed classification.
- T7: owner-local `bulk_auth_wire_pairs_and_positive_predicates` injects facts,
  never Findings, into a real server and proves all three auth pass/fail/unknown
  cases with complete source hints. Public `bulk_untrusted_auth_headers_stay_unknown`
  pins exact reports despite spoofed headers and absent/duplicate From fields.
- T8: existing REST/MCP and stdio parity tests now exercise the bulk report.
  Public thresholds exclude unknowns at info/warning/error; exact zero and
  minimum-one waits retain their meanings. Trusted owner fixtures exercise
  positive qualification, paging and the UI's empty-report state. Living README,
  llms.txt and OpenAPI descriptions state scope, unsupported syntax and bounds.
- T9: existing cancellation/cache/deadline/subscribe-before-scan tests remain.
  The held-worker real-server proof additionally delivers SMTP mail, observes
  queryability and oldest pruning, and deletes mail before releasing workers.
  SMTP/seed/restore still create no analysis slots.
- T10: fresh gate commands, exits, log SHA256s, tested commit/tree and exact
  coverage summaries are recorded in ignored `.forge/epic192/issue257/` evidence.
  The candidate-local pinned engine compiles the existing self bundle. No recipe,
  dependency, license allowance, accepted ruling or historical spec is changed.

Both 0085 and 0089 remain proposed. CI owns macOS, network RustSec, Helm/chart,
Docker, privileged port-80 and tag proofs; the operator owns settings, decision
acceptance, branch sealing and publication. Local results make none of those
claims. No push, merge, tag, publication or issue closure is performed.
