# Decision-record repair for #184

Created: 2026-10-24. Status: draft design for implementation.
Story: epic #192, issue #184, five scoped decision-record findings.
Run: epic-192-issue-184-repair-the-fi-984472b4.

## Feature and rationale

Record the operator's supplied acceptance of the exact corrected 0051
proposal, reconcile its two predecessor records and index, and append four
factual corrections. Operators and future delivery seats must identify the
current release and protocol contracts without mistaking a preserved
historical statement for current authority. No runtime, release configuration
or publication implementation is required.

The authority is the operator instruction supplied to this run, identifying
`.forge/epic192/proposals/0051-release-records.md` with SHA-256
`89fff17e5a160de79e837bb0e61b2f00bfeef85241d515d1a18e7679f9f257dd`.
The hash was independently checked during design. Record the supplied
acceptance on 2026-10-24; do not invent an earlier ruling timestamp or infer
acceptance from shipped code or the 2026-10-19 acceptance batch.

The normative amendment already has its owning record,
[0051](../../docs/decisions/0051-release-records.md), whose tracked status is
still proposed at this design commit. Implementation records the supplied
acceptance there. This design proposes no additional semantics, so no fresh
ADR number or decision declaration is introduced: duplicating the ruling
would exceed the specifically authorized decision set. Other authority:
[0019](../../docs/decisions/0019-release-tag-integrity.md),
[0045](../../docs/decisions/0045-delivery-repairs.md),
[0056](../../docs/decisions/0056-signing-policy.md), and
[0057](../../docs/decisions/0057-codex-delivery-recipes.md).

## Scope and history

Implementation may change only these existing files:

- `docs/decisions/0051-release-records.md`
- `docs/decisions/README.md` (0051 row only)
- `docs/decisions/0019-release-tag-integrity.md`
- `docs/decisions/0045-delivery-repairs.md`
- `docs/decisions/0028-wire-octets-auth-refusals-and-the-maildrop-snapshot.md`
- `docs/decisions/0030-loopback-plane-bounded-results.md`
- `docs/decisions/0007-hermetic-gates-single-model.md`
- `docs/decisions/0023-off-worker-commits.md`
- `tests/store_contract_docs.rs` (only necessary scoped provenance proof).

This design's three committed spec files are delivery artifacts, separate
from that implementation allowlist. Positions and gate receipts stay
run-local. Preserve accepted historical bodies and titles; corrections are
dated appendices. Preserve the supplied 0051 text except its current status
metadata and added adoption provenance. No other decisions, house rules,
source, manifests, release notes, chart, CI, gate machinery, dependencies or
allowlists change. No push, merge, tag, publication or registry action.

The earlier triage's prohibition on 0019/0045 edits is superseded by this
run's explicit authorization. Its stopped attempt remains history; no
implementation or gate success is inferred from it. The active framing is
`.brokkr/state/tasks/epic-192-issue-184-repair-the-fi-984472b4.md`.

## Panel synthesis and rulings

Both run-local positions were read in full. They agree on the substantive
repair; there is no genuine opposing release-policy recommendation to
average. The chief makes the following explicit choices.

1. Adopt simplicity's existing owners and eight-file documentation boundary.
   Adopt robustness's mandatory accepted provenance and predecessor pointers.
   Reject a status-only flip: its lower edit cost loses a coherent current
   contract and breaks the existing proof. The extra cost is two dated
   backlinks and narrowly maintained test fixtures.
2. Adopt both positions' preservation of the exact proposal and dated history.
   Reject silently replacing old text or removing pending-language paragraphs.
   The accepted cost is that readers must follow the current header and
   adoption appendix; that appendix must explicitly supersede historical
   awaiting-operator and conditional-adoption language.
3. Adopt robustness's release failure distinctions and simplicity's refusal
   of additional machinery. Reject permanent preparation equality, a release
   coordinator, ledger, automatic recovery or digest attestation. The costs
   remain unreleasable preparation trees, partial image-only publication,
   registry outage refusal and a check-then-push race. Acceptance is not a
   release receipt and does not solve these residuals.
4. Adopt the finite existing provenance checker and preserve its negative
   proofs. Reject deleting tests to retain a documentation-only label, and
   reject a general ADR parser or duplicated suite. Human review still owns
   semantic truth and operator provenance; structural tests cannot certify
   either. Robustness's broader runtime failure analysis is preservation
   context, not permission to add runtime work or new guarantees.

## Acceptance Criteria

- **AC1 — Scope and preservation.** The implementation diff contains only
  the nine allowed paths. The six predecessor/factual records are append-only;
  README changes only the 0051 row. Against the hash-pinned proposal, adopted
  0051 differs only in current status/original-status provenance and a dated
  adoption appendix. A full diff and link audit proves this; no historical
  version table is rewritten as a current version-selection instruction.
- **AC2 — Adoption authority.** 0051 and its index both say accepted, attribute
  acceptance to the supplied operator ruling recorded 2026-10-24, identify
  the exact proposal/hash, and retain original draft status. The appendix
  expressly makes all preserved awaiting/operator-choice/conditional-adoption
  language historical. No seat self-acceptance or backdated batch acceptance.
- **AC3 — Predecessors.** 0019 and 0045 gain dated linked accepted
  `Amended/clarified by 0051` markers. They supersede the pending status of
  the retained 2026-10-19 proposal pointers and qualify exactly 0019's
  four-way-publication-equality rejection and 0045 §5's independent
  publication interpretation, without rewriting their bodies.
- **AC4 — Release boundaries.** Successful joint image-and-chart tag releases
  require tag version = Cargo package version = Chart appVersion = Chart
  version; default rendered/Artifact Hub images agree with appVersion.
  Preparation chart versions may differ. Retain explicit Docker success
  before chart publication, present-image check, absent chart tag, both image
  tag absence probes, exact-ref guards, unknown-state refusal, ordinary
  prerequisites, same-run rate proof, chart event-history proof and main's
  separate image role. Retain partial-release/operator-recovery and old-rerun
  limits. 0056 still governs unsigned authoring, operator seal and separately
  signed release tags. No transactionality, immutable tags, digest proof or
  external-action permission is implied.
- **AC5 — AUTH/USER correction.** 0028's dated correction qualifies both §2
  and its old 0046 pointer: disabled SMTP AUTH is 502; transaction/repeated
  AUTH is 503 before decoding; eligible malformed credentials are 501 and
  preserve identity. It does not claim every AUTH refusal is 501 (unsupported
  mechanisms can be 504). Failed POP3 USER in authorization clears pending
  identity, answers -ERR, and subsequent PASS requires USER. Preserve one
  successful AUTH per TLS epoch and strict verbatim names under 0034/0046;
  no trimming or new authentication semantics.
- **AC6 — Limit correction.** 0030's dated correction explicitly qualifies
  both §3 and Consequences' every-limit maximum claims. Applicable page/wait
  domains retain 250; assert count is exact with zero allowed and no page cap;
  assert limit is unsigned, type-checked and inert with no page maximum or
  default. Parser numeric bounds remain; no arbitrary-precision promise.
  Cite the served `src/static/openapi.json` and existing handler/schema proof.
- **AC7 — Missing history pointers.** 0007 gains a dated marker linking
  accepted 0013's judging-axis amendment, distinguishing 0013's 2026-10-13
  acceptance from insertion on 2026-10-24. Preserve later 0057 route authority,
  hermetic exec seats and protected review/security hold. 0023 gains a dated
  factual correction linking watcher lifetime/atomic registration to 0021;
  0010 remains exact counts and 0023 remains off-worker commits. No concurrency
  ownership or mutation-order change.
- **AC8 — Discriminating provenance proof.** The existing documentation suite
  passes with both accepted 0051 relationships. Removing either new accepted
  backlink while leaving the historical proposed marker must fail. Missing
  successor declaration, index/status mismatch and a fixture with proposed
  status plus claimed accepted marker still fail. The forged-acceptance proof
  must not depend on live 0051 remaining proposed. Preserve all other audited
  relationships and refusal cases; no generic prose validator.
- **AC9 — Delivery proof.** Run `cargo test --locked --test store_contract_docs`
  and all gates in [house rules](../../docs/house-rules.md#gates--part-of-every-change)
  on the final implementation, including coverage selftest, publish/seal
  proofs, locked commands and fail-closed native-tls absence. Exact coverage
  remains 100/100/100. Existing SMTP/POP3/API and served-schema tests establish
  behavior. Retain the engine-selected full four-lens review with official
  DeepSeek Flash and DashScope GLM 5.3, no Pro or fallback. CI-only evidence
  keeps its assigned owner; unavailable required proof remains outstanding.
- **AC10 — Honest completion boundary.** Do not claim this adoption proves a
  guarded registry release, packaging attribution, or closes the separate
  #169/#185/#186 obligations. No frozen guarantee or ingest-time extraction
  changes. Chief returns only drafted; the pinned final design check owns its
  verdict, and implementation/verify/review remain downstream.
