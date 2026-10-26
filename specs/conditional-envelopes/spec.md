# Qualify Outlook conditional envelopes

Date: 2026-10-26. Status: implemented and proved locally; dated evidence is in plan.md.
Protected verification and review remain engine-owned.
Scope: the R4.4 follow-up to run `implement-the-operator-s-2026-10-eec6b2f0`,
whose review ruled REVIEW-RESIDUAL-OK. This design addresses correctness
MEDIUM 1 and LOW 2 / guarantees G1 in the supplied review.

## Feature and rationale

A fresh message grants source-aware quoted-attribute handling only to a
completely qualified Outlook conditional comment. A malformed comment keeps
ordinary-source URL delimiters. This distinction changes stored answers:
`href="https://x.io/O&#39;B"` yields `https://x.io/O'B` in a qualified
conditional payload, but `https://x.io/O` in an ordinary comment. Exact
arrays must distinguish these outcomes; returning both is not a repair.

The current extractor checks the opening `[if` and header terminator but
never qualifies the final marker. Its no-marker inline fixture consequently
expects the wrong full URL. Its byte-partition assertion also cannot detect
repeated traversal of a suffix. Repair both findings in the existing lexical
owner, with regression-first behavior and actual-work observations.

Authority is the operator's 2026-10-26 ruling recorded in accepted
[0061](../../docs/decisions/0061-compat-webhook-proof.md#operator-acceptance-and-decisions--2026-10-26)
and the dated [0049 amendment](../../docs/decisions/0049-extraction-compat.md#6-amendment-and-conditional-source-ruling--2026-10-26).
The lexical qualification already appears in
[the preceding plan, steps 3–7](../compat-rulings/plan.md#implementation-approach).
This is conformance to those requirements, not a change to a ruling or public
contract; no new decision or acceptance is proposed. Accepted
[0040 §6](../../docs/decisions/0040-extraction-fidelity.md) requires observable
work; [0005](../../docs/decisions/0005-extraction-at-ingest.md) requires fresh-ingest
extraction and verbatim historical fields.

## Panel synthesis and rulings

Both panel positions were read in full. They agree on raw qualification,
bounded payload scanning, ordinary fallback and measured work; there is no
substantive semantic conflict to invent. The following choices resolve their
implementation emphasis and the chief's artifact ownership.

| Position | Ruling | Cost of the selected approach / rejected alternative |
|---|---|---|
| Simplicity: reuse `markup_at`, `tag` and `LinkRegions`, with local inline and wire tests. | Adopt. No new parser, production module, fixture framework or dependency. | The lexical heuristic remains smaller than browser parsing; conditions stay opaque and malformed comments can retain truncated links. |
| Robustness: qualify the entire envelope before callbacks and return bounded source offsets. | Adopt as mandatory, reconcile with simplicity through a small local range qualification step. A suffix-only patch without header/payload ordering is rejected. | A few linear passes remain; callbacks cannot be speculative because `LinkRegions` already appends results. |
| Both: instrument real loops, not just byte partition. Robustness emphasizes proof breadth. | Adopt explicit local counters and a repeated-work negative control; reject partition-only proof or elapsed-time tests. | Counters and edge cases add maintenance and exact-coverage obligations. Cutting them would leave the same LOW finding open. |
| Simplicity: reuse old spec records; robustness: retain scoped, candidate-bound evidence. | Keep inherited records historical; commit one new `conditional-envelopes` spec directory and keep its proof summary current. | One more small spec directory costs navigation, but avoids rewriting the preceding run's broader requirements or misidentifying the chief's design. |
| Robustness: account for storage, webhook, chaos and concurrency failure limits. | Preserve these as constraints and existing-suite controls, not new subsystems or acceptance promises. | This repair does not remove best-effort notification loss, parse-pool pressure, retry ambiguity or cap eviction. |

## Acceptance Criteria

1. **AC1 — Complete raw qualification.** Use the established first raw comment
   close; never search for a later close to make a marker fit. Immediately
   after `<!--`, require ASCII-case-insensitive `[if` followed by ASCII
   whitespace. Require raw `]>` ending the opaque header before or exactly
   at the payload end. After skipping only trailing ASCII whitespace in the
   body, require the nine-byte ASCII-case-insensitive `<![endif]`. Payload
   begins after the header and ends before that marker; it may be empty.
   No leading whitespace before `[if`, `[iffy]`, encoded marker, Unicode-only
   whitespace substitution, missing header terminator, missing end marker,
   missing outer close or existing abrupt close qualifies. Ordinary-comment
   and unfinished-tail behavior is preserved on every failed qualification.
   A marker followed by non-whitespace content is not final.
2. **AC2 — Discriminating regression-first evidence.** Before production edits,
   add the three individually runnable tests named in [tasks.md](tasks.md):
   missing marker, mixed-case marker and marker not final. Missing/non-final
   cases must fail behaviorally on the baseline with exact expected arrays.
   Correct the existing no-marker expectation. Mixed case is valid, not a
   rejection case: record its actual baseline outcome, then prove a temporary
   case-sensitive final-marker mutant fails that test after qualification
   exists. Restore exact bytes and prove all three green. Commands, nonzero
   counts, assertion failures, exits and source/log hashes are required;
   compilation errors, zero-test selections and expected-failure annotations
   are not behavioral red evidence.
3. **AC3 — Bounded source ownership.** No `Regions::value` call precedes full
   qualification. Report only closed quote-after-equals attribute ranges
   inside the qualified payload, in source order. Reuse existing tag rules,
   including quoted `>`. Completed tags advance past their whole extent;
   an unfinished tag/quote stops the inner walk, retaining previously closed
   values and never restarting at later `<` bytes. Do not recurse into
   declaration-like markers. Unassigned source keeps ordinary link handling.
   Exact arrays cover multiple attributes, non-ASCII text, encoded delimiters,
   nested URL punctuation, decode-once, empty payload and malformed tails.
   A following outside anchor remains visible. Comments emit no codes and
   inner tags never change outer head/template/raw-text suppression. The
   supplied O'Brien fixture remains a complete-URL singleton both alone and
   with its duplicate ordinary anchor.
4. **AC4 — Actual work and progress.** Instrument actual opener/header/end
   qualification, payload search and tag traversal, including unsuccessful
   searches. The categories and explicit linear bounds in [plan.md](plan.md#work-observations)
   must hold on deterministic increasing sizes, repeated malformed openers,
   invalid/non-final markers, non-ASCII bodies and unfinished attributes with
   many later `<` bytes. Exact small-fixture counts, monotonic progress,
   bounded marker work, one decode per processed link region and
   `ordinary_bytes + quoted_bytes == input_bytes` must be asserted. A
   temporary repeated-suffix-work mutant must fail the work assertions while
   leaving URL output and partition unchanged; restore source byte-for-byte
   and rerun green. Preserve unrelated counter pins; no process-global state,
   timing bound, coverage exclusion or denominator reduction.
5. **AC5 — Stored behavior and unchanged rulings.** Real SMTP/HTTP controls use
   isolated authenticated inboxes and both shim views. After DATA 250, assert
   exact stored links/codes and unchanged raw/body truth for qualified,
   missing-marker, mixed-case and non-final cases. Assert stored arrays through
   the native stored-field response and verify each shim's existing projection;
   do not add response fields to a shim to expose these assertions. Existing
   complete-URL, nested-URL, independent-MIME text-first, #136 fidelity and
   historical live/reopen tests pass. Old arrays, even wrong ones, remain
   verbatim. Diff review confirms no change to refusal recognition, MIME
   ordering, persistence, SMTP acceptance, watcher registration, pruning,
   routing, gates, dependencies or recorded decision rulings.
6. **AC6 — Committed final proof.** Complete every local house gate, exact
   zero-miss line/branch/function coverage and its selftest, and configured
   supplemental proofs on the final implementation. Use the digest-checked
   candidate engine, offline installed tools and current bundle routing.
   Bind commands/log hashes to source/tree identities; restore all mutants,
   reconcile this task ledger with actual selectors and commit all scoped
   deliverables unsigned. Missing prerequisites/failures are named precisely,
   never waived. Protected verification and review remain engine-owned.

## Scope and limits

Production edits belong only in `src/extract.rs`; new deterministic tests
remain inline, wire controls in `tests/compat.rs`. Existing code, storage and
protocol suites protect frozen 0001–0003 without changing their algorithms.
There is no condition evaluator, DOM recovery, URL post-filter, link cap,
background extraction, migration, read-time repair or broader historical-ledger
cleanup. The prior review's shim-result-ID LOW and unrelated task-ledger LOW
remain outside this same-code follow-up with their existing dispositions.

CI owns macOS/platform evidence, chart/history checks, Docker smoke,
network-backed RustSec and privileged port-80 tests. Live GitHub settings,
namespace provisioning, release/tag proofs, sealing and publication remain
CI/operator responsibilities, not local acceptance claims. No network lookup,
new decision acceptance, push, merge, tag, publication or issue closure is
part of this delivery. Historical wrong arrays remain wrong by design.
