# Issue 182 residuals: truthful refusal guidance and executable limits

Date: 2026-10-24. Epic #192, issue #182. Baseline:
`72f489a86f2e395e4ce691eeef961aacf7ab0112`.

## Feature and rationale

Close three recorded review gaps: document the existing malformed seed fixture
exception beside the supplied `timeout_ms` exception; exercise the actual
thread grouping and returned keys for `ß` versus `ss`; and replace ineffective
issue-182 documentation assertions with checks demonstrated to detect the
specified defects. Readers must be able to distinguish protocol errors from
tool failures without inferring a different API from the documentation.

This is a documentation and test correction. All five existing issue criteria
remain obligations. Implementation is limited to `README.md`,
`src/static/llms.txt`, the issue-182 checks and their local helpers in
`tests/store_contract_docs.rs`, and inline `#[cfg(test)]` tests in
`src/threads.rs`. Production behavior, dependencies, accepted decisions,
gates, shims, recipes and unrelated text remain unchanged.

## Authority and semantic classification

No new decision is declared: no public behavior or accepted ruling is amended.
The missing seed exception restores [0010 §2](../../docs/decisions/0010-exact-count-asserts-and-the-shared-seed.md),
which explicitly requires a field-naming MCP `isError` refusal before insertion.
The shared constructor and MCP adapter already implement it. Ordinary argument
validation remains governed by [0029 §2](../../docs/decisions/0029-supplied-values-are-validated-wherever-they-reach-a-wire.md)
and [0044 §3](../../docs/decisions/0044-compat-validation.md).
Wait bounds and refusal envelopes follow [0018](../../docs/decisions/0018-shutdown-observed-waits-and-explicit-bounds.md)
and [0025 §4](../../docs/decisions/0025-shared-domain-validation.md).
The REST data/extractor split follows [0048 §2](../../docs/decisions/0048-rest-webhook-validation.md);
POP3 follows [0028 §3](../../docs/decisions/0028-wire-octets-auth-refusals-and-the-maildrop-snapshot.md).
These are authority links, not proposed decisions or newly accepted rulings.
If implementation discovers a genuine contradiction with an accepted ruling,
return that evidence through triage; this spec grants no semantic expansion.

## Panel synthesis and rulings

Read both run-local positions, `simplicity.md` and `robustness.md`, and intake
`.brokkr/state/tasks/epic-192-issue-182-residual-foll-2557ad82.md`.
Their provenance is run-local evidence, not an additional committed artifact.
The positions agree on architecture and four-file scope; there is no genuine
runtime design conflict to average or manufacture.

- Adopt simplicity's reuse of the existing seed owner, wire regression,
  thread helpers and documentation test file. Reject a schema framework,
  Markdown parser, new fixture factory, Unicode dependency or extra HTTP
  threading test. The cost is bounded proof of the named defects rather
  than arbitrary prose equivalence or every Unicode mapping.
- Adopt robustness's stronger defect controls: independently challenge the
  introductory exception and the detailed seed sentence, including the
  field reason/no-insertion promise and unknown-key rule. This rules in
  favor of a few more local test cases over the smallest possible edit.
  The losing minimal-edit option saves maintenance but can certify an
  internally contradictory guide. Keep simplicity's small local predicate
  approach; a generalized mutation framework is rejected.
- Adopt both positions' real grouping test with actual returned keys and
  membership. Reject a comment-only, normalize-only or expected-hash-only
  proof. The extra deterministic assertions are the necessary cost of
  detecting a merge or returned-key conflation.
- Retain robustness's failure ledger as preservation constraints, not new
  implementation work: synchronous acceptance, subscribe-before-check,
  oldest-only pruning and ingest-time extraction remain intact. No new
  persistence, chaos, TLS or webhook mechanisms/tests belong in this patch.
  Existing finite retention and best-effort webhook loss remain accepted
  limits; this repair promises neither exactly-once delivery nor a durable
  outbox, snapshot lock or broader content sanitization.
- Retain recorded triage `engine`. Under [0057](../../docs/decisions/0057-codex-delivery-recipes.md),
  its four configured independent lenses are the smallest review allowed
  by this selected route. A smaller diff does not silently reclassify it.
  The cost is panel latency. The engine owns that later review; this chief
  launches no substitute reviewers, uses no Pro/fallback, and selects no
  next phase.

## Acceptance Criteria

1. **AC1 — Precise seed exception in both guides.** The general MCP
   malformed-argument rule explicitly exempts both supplied wait `timeout_ms`
   refusals and malformed seed fixture content. A coherent sentence names
   `swarmail_seed_email`, CR/LF/NUL in `to`, `from` or `subject`, an HTTP-200
   tool result with `isError: true`, field-specific reason in text content,
   and no insertion, citing decision 0010. HTTP response claims apply to
   calls with an `id`; notification silence is preserved. Wrong-typed seed
   arguments and unknown keys are not included in the content exception.
2. **AC2 — Preserve issue criteria 1 and 3.** Both guides bind unknown MCP
   argument keys to JSON-RPC `-32602`, HTTP 200 for an identified call,
   before any effect. They distinguish supplied malformed/out-of-range
   `timeout_ms` (integer domain 0 through 60000, `isError: true` refusal)
   from an elapsed valid wait (`isError: false`, `matched`, `total`,
   `timed_out: true` in text content), runtime tool failure such as
   `shutting down`, and the existing protocol `error` envelopes. Ordinary
   argument errors stay `-32602`; existing parse/body/notification rules
   stay unchanged. No new precedence rule for multiply-invalid calls is
   implied.
3. **AC3 — Preserve issue criterion 2.** Both guides retain seed/webhook
   JSON data errors as HTTP 400 with JSON `error`, distinct from extractor
   syntax errors (plain-text 400), missing/wrong media type (plain-text
   415), oversize body (plain-text 413) and chaos JSON data errors (422).
4. **AC4 — Prove issue criterion 4 through actual threading.** README keeps
   the Unicode lowercasing versus full-case-folding limitation. One inline
   deterministic regression groups two mails with subjects `ß` and `ss`,
   distinct IDs and fixed unequal timestamps, with no Message-ID,
   In-Reply-To or References. It asserts two conversations, count one and
   expected member ID in each, and unequal keys from the returned threads.
   A merge or identical returned keys fails. The existing Über/über
   positive test remains. No production function or test-placement change.
5. **AC5 — Preserve issue criterion 5.** README states that POP3 listings
   use the successful-PASS snapshot and stable numbering; new arrivals
   appear next session. RETR/TOP/DELE consult live storage, so mail removed
   elsewhere can remain listed and yield `-ERR no such message`.
6. **AC6 — Documentation checks discriminate.** The five issue-182 checks
   validate the owning complete clauses in both applicable guides (README
   only for the existing thread and POP3 claims). Reuse those same checks
   on independently edited in-memory documents. Each edit must demonstrably
   change its input and fail its owning checker, with corrected originals
   passing before and after. Required defects: remove only the introductory
   seed exception while preserving its detailed sentence; remove only the
   detailed sentence; relabel seed-content refusal as `-32602`; remove its
   field reason or its no-insertion promise separately; conflate supplied
   timeout with protocol error; label elapsed wait `isError: true`; remove
   the unknown-key/-32602 guarantee; restore the blanket seed/webhook body
   error claim; remove the subject limitation; remove the POP3 snapshot/live
   storage qualification. Apply each control to every document containing
   the corresponding claim. Whitespace wrapping and ASCII capitalization
   changes preserve verdicts for both valid and invalid fixtures. Delete
   or replace the implicated vacuous negatives; a substring within valid
   `Other protocol and argument refusals` prose is not a forbidden claim.
   Do not change the unrelated shared `states`/`refuses` behavior.
7. **AC7 — Retain behavioral evidence and scope.** Execute the existing
   `hostile_envelope_fields_are_refused_on_both_surfaces` real-server test:
   REST 400, MCP HTTP 200/`isError: true`, field reason and unchanged inbox
   counts. Execute the complete threading and documentation suites. Diff
   inspection confirms only the four permitted implementation files and
   only inline test code in `src/threads.rs` changed after this design.
   No mocks, new dependencies, exclusions or lowered coverage denominator.
8. **AC8 — Delivery remains gated.** All gates in
   [the single house inventory](../../docs/house-rules.md#gates--part-of-every-change),
   including the coverage selftest, publish-guard and seal proofs, pass on
   the implemented candidate, with fail-closed native-tls absence proof.
   Exact line/branch/function coverage stays 100%. Pinned tooling and shim
   checks apply; CI-only proofs remain CI-owned and cannot be inferred from
   local passes. Configured independent review and security holds remain
   mandatory. No push, merge, tag or publication.

## Dated clarification — 2026-10-24

This chief commits the design only. Ordered implementation tasks remain open;
source inspection or a baseline gate run cannot certify the missing fixes.
The final deterministic design check owns its verdict on the committed
artifacts. The chief reports only `drafted`, not implementation completion.
