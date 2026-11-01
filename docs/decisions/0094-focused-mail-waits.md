# 0094 — Ingest provenance and focused mail waits

- **Status:** proposed
- **Date:** 2026-10-28
- **Scope:** Epic #164 issue #154 on provisional #153 commit 25b367d.
- **Specification:** [Focused mail waits](../../specs/focused-mail-waits/spec.md).
- **Plan and proofs:** [plan](../../specs/focused-mail-waits/plan.md),
  [tasks](../../specs/focused-mail-waits/tasks.md).
- **Supplements:** the public tool inventories of proposed
  [0082](0082-mcp-structured-output.md) and [0105](0105-mcp-report-output.md);
  extends ingest-owned extraction under [0005](0005-extraction-at-ingest.md)
  compatibly, without changing existing stored fields or historical restore.
- **Preserves:** 0001–0005, 0010, 0030/0031, 0048/0049/0061, 0074 and the
  current report contract; 0079/0080 deviations and proposal statuses unchanged.
- **Number:** provisional next free after checked local branch-tip allocations.
  #155 owns proposed 0092-compact-email-view; the OPTIONS follow-up also carries
  a different 0092, an existing collision. #156 owns 0093-near-miss-timeouts.
  No local branch tip exposed 0094. Recheck before integration; only the operator
  resolves collisions/accepts decisions. Do not reuse historical #154/0084.
  *(The #155 and #156 numbers above are superseded: see the
  [dated correction of 2026-11-01](#dated-correction--2026-11-01-decision-numbers).)*

## Context and authority

The checksum-verified packet and intake commission ingest-time link anchor text,
rel/origin plus focused code/link waits with explicit provenance and ambiguity.
The later 2026-10-26 Phase 1 direction permits development on #153 now, with #151's
operator-owned paid baseline still a merge prerequisite. The current base is
commit 25b367de5e7920666bebd96a7c5216f60c8cfd1a, tree
91c8ada3f36d396e824c06f800f6a501bf577bec. It is provisional, not reviewed #153.

The spec records verified packet hashes and historical patch SHA-256
e84cfe9c8049c0e43ba6b664838a13f6f5b5d8eeb36af1569ff932d5610cd691.
The historical five-member review identified mixed-attribute ownership, stale
terminal snapshots and nondeterministic/weak wait/filter proofs. Later historical
repairs inform this design but establish no current verdict.

The operator requires historical specifications to remain historical. This new
record supplies current corrections without editing old #152/#153/#154 specs or
claiming that their older inventories describe the combined tree. Packet-recorded
acceptance provenance for 0079/0080/0082 remains distinct from locally proposed
headers; this change accepts no record. Compatible implementation can proceed
under [0045 §8](0045-delivery-repairs.md), never contradicting accepted rulings or
weakening frozen guarantees.

## Proposed contract

1. Store internal ordered link occurrences with required url, nullable anchor_text
   and rel, and text/html source, extracted by the existing source-aware ingest
   tokenizer before synchronous insertion. Preserve repetitions and source identity
   through quoted AND ordinary regions; an unquoted href preceding quoted rel/title
   cannot lose ownership. First href/rel wins and lexical labels follow the spec's
   decode/whitespace/suppression/boundary rules. Legacy links remain first-occurrence
   exact-string projection, text first. Preserve current MIME/codes/raw/bodies.
2. Skip the new optional vector in existing Email serialization on all surfaces.
   Fresh Some([]) is known empty; historical SQL NULL is unavailable. Add one
   nullable persistence column in the same insert/prune/counter transaction. Never
   re-extract/backfill historical rows. Non-NULL malformed metadata, including
   missing nullable members or extra members, refuses restore explicitly. Existing
   old-field fallback behavior is outside this change.
3. Add swarmail_wait_for_code and swarmail_wait_for_link, retaining current inbox,
   to/from/subject/since_ms and 0..60000 ms timeout domains/defaults. Validate closed
   input shapes and all selectors before parking/subscription; preserve -32602
   versus text-isError taxonomy and value-free named refusals. Link regex searches
   are at most 1024 UTF-8 bytes and 1 MiB compiled each, ANDed on one occurrence.
   Host/path come only from local parsing of stored URLs; no network/trust behavior.
4. Use a single focused wait owner beside the unchanged report wait owner. One
   monotonic deadline, subscribe before capture, resubscribe before capture after
   Closed, rescan after lag, retain receiver during offloaded eligibility, no
   guards across work, and current adapter shutdown/readiness apply. Classify a
   scan terminal before capture. An unsuccessful nonterminal scan crossing expiry
   requires one fresh terminal scan; a terminal scan terminates once. Zero timeout
   scans once. Report expiry/ready-only/cold-analysis/failure behavior stays intact.
5. Code selection uses native acceptance order, including persisted sequence on
   restore. One distinct code in the newest eligible message selects; several are
   ambiguous with no fallback. Include ordered older candidates with email ID,
   received_ms, saturating age and stored sender. Sender can include existing
   envelope fallback; age is parse-start wall time, not expiry or acceptance order.
6. Link selection examines every matching occurrence across the full retained
   filtered snapshot: one distinct exact URL selects, several expose ambiguity.
   Preserve repeated provenance; historical URLs have unknown label/rel/source.
   Cap displayed lists independently at 250 AFTER semantic evaluation, with exact
   totals/truncation. The spec defines closed selected/ambiguous/timed_out fields;
   ambiguous/timeout omit selected values. Keep same-snapshot matched count and
   omit the historical independently sampled unfiltered inbox total.
7. Fifteen tools, including deliverability, have complete object-root outputSchema
   on June/November and successes carry matching text/structuredContent from one
   captured Value. New tools are read-only. Existing March/default result bytes
   and descriptor entries remain unchanged; discovery adds the two entries.
   Preserve independent captures and validate the original descriptor subsequence.
   March additions remain text-only. Retain report schema bounds/unions/OpenAPI
   pins and finite evaluator refusal of unknown assertions. No runtime validator,
   retry, schema generator or dependency is added.

## Explicit synthesis and rejected alternatives

Choose simplicity's internal metadata over robustness's public modern Email
projection: old general reads expose only URLs; users need the new tool for
labels. This deliberately avoids REST/webhook/March projection changes and their
additional proof obligations. Historical broad serialization is not authority
to change fresh March payloads or rewrite goldens.

Choose robustness's narrow focused path over simplicity's shared lifecycle
refactor. Duplicating a short wait loop costs maintenance and paired regressions,
but protects current lazy report qualification and expiry behavior that the
historical patch predates. No second watcher registry, polling or wait service.

Adopt both panels' source ownership, full-snapshot ambiguity and terminal scan;
adopt robustness's strict restore/negative/interleaving/filter proofs. Reject
cursor/equal-URL attribution, first-page selection and wall-clock scheduling
assumptions even when they make the patch smaller. Pay source bookkeeping, one
extra terminal scan and corruption refusal rather than plausible wrong answers.

Adopt simplicity's finite schemas/existing libraries and omit independently
observed unfiltered totals. Reject a DOM, trust service, durable outbox, background
extraction, compact view or near-miss framework. Extra framework coverage, public
vocabulary and operations buy no framed requirement.

## Dated historical corrections — 2026-10-28

The supplied old #154 patch's public Email enrichment, header-only sender claim,
shared wait replacement and unfiltered total do not govern this candidate. Here
metadata is internal; sender is the actual stored field including current fallback;
focused waiting coexists with reports; totals describe the decision snapshot.
The thirteen-tool report supplement becomes fifteen through two additive tools.
Old specs, decisions, captures and reviews stay intact; this is a new proposal,
not replacement or acceptance of historical 0084 or #155's pending 0092.
*(Superseded number: #155's record is now
[0108](0108-compact-email-view.md); see the 2026-11-01 correction below.)*

## Costs, proof and disposition

Metadata adds ingest/storage cost. Conservative lexical labels are not browser
rendering, trustworthy destinations or prompt-injection protection. Historical
unknown provenance cannot satisfy an anchor filter. Complete snapshots and exact
URL sets cost memory/CPU; 250 entries is neither a byte nor global-memory bound.
Terminal work can finish after timeout and observe a later commit. Shared pool
contention and started work after cancellation remain. No retention reservation,
code-validity/intent inference or global concurrency guarantee follows.

Preserve transactional accept-before-250 and best-effort webhooks; a post-commit
lost reply can cause a duplicate resend, memory-only state dies with the process,
and notifications may be lost. Both modern JSON representations can exceed the
unchanged 10 MiB bridge cap; preserve correlated refusal/continuation, never drop
one representation or retry a mutation. TLS validity remains distinct from loadable
configuration and existing generated-fixture proof discipline applies.

AC1–AC10 and paired tasks require exact extraction/work/migration proofs,
deterministic lifecycle schedules, independent wire filters, all-tool/revision/
transport/schema/legacy evidence and real application stale-code/decoy controls.
Run every house gate locked as applicable, exact coverage/selftest, pinned
publish/seal/design/delivery proofs, licenses and native-tls absence with checked
shims and digest-verified .forge/bin/brokkr. Current logs/tree identities alone
bind current evidence. A design check is not implementation verification.

Reconcile eventual reviewed #153, separate #152 OPTIONS, current main and numbering
before final delivery. Carry the inherited HIGH security-relevant review-integrity/
result-attribution residual unchanged; no attack reproduction, protected-boundary
widening, engine re-pin or unpublished-fix claim. Terra 6.1 intake, Opus 5.5
implementation and the inherited five protected roles retain ownership, including
Astra correctness/guarantees, independent Claude, official Flash security and
DSH Spark GLM-5.3-Flash tests. This record supplies no verdict for those seats.

Standalone eval suite-wide coordination stays unproved under #201; no private
lock or fabricated receipt substitutes. #151 paid baseline, actual clients, CI
and operator seal remain independent prerequisites. No paid trials, push, merge,
publication, tag, issue closure or decision acceptance. Chief returns drafted;
the deterministic final check and subsequent phases own advancement.

## Dated correction — 2026-11-01 (decision numbers)

The Number line and the last sentence of the 2026-10-28 corrections record the
branch-tip allocations seen while this record was drafted; their #155 and #156
numbers no longer hold on `main`.
#155's compact email view is proposed
[0108](0108-compact-email-view.md) (drafted as 0092), and #156's near-miss
timeouts are proposed [0111](0111-near-miss-timeouts.md) (recorded above as
0093). On `main`, 0092 is [official-mail-comparison](0092-official-mail-comparison.md)
and 0093 is [source-mail-comparison](0093-source-mail-comparison.md), both from
#163; the OPTIONS lineage's 0092 (mcp-candidate-proof) is
[0104](0104-mcp-candidate-proof.md). 0094 itself is unchanged on `main`. The
historical sentences are kept as written; no ruling or status changes.
