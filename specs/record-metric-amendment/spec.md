# Record the decoded-inbox metric amendment

Epic #192 / issue #202. Design synthesis, 2026-10-24.

## Feature and rationale

Record the operator's existing acceptance of the decoded-inbox request-metric
amendment as 0065, with accurate provenance and scoped historical pointers.
The task supplies the operator's chat ruling, “accept that”, dated 2026-10-24,
and the reservations #180=0062, #181=0063, #174=0064. This is authority to
record that acceptance only; no independently inspected transcript is claimed.

The existing `docs/decisions/0062-decoded-inbox-metrics.md` incorrectly uses a
reserved number, remains proposed and has no index row. Accepted
[0043 §6](../../docs/decisions/0043-mcp-origin-boundary.md) and its HTTP-plane
spec/plan preserve raw request keys. The amendment's own design still demands
acceptance. Renaming and changing status alone would leave those active
references contradictory. Preserve history while making the current authority
and its narrow scope discoverable.

This design introduces no new ruling or proposed decision. The semantic
ruling already exists and the operator has accepted it; implementation of
this documentation task records it at `docs/decisions/0065-decoded-inbox-metrics.md`.
Creating another proposed ADR would duplicate authority and misstate the task.
Under [0045 §8](../../docs/decisions/0045-delivery-repairs.md), design validation
is never acceptance. This design commit contains only this spec, plan and tasks;
the later implementation phase performs the bounded documentation changes.

## Requirements

Rename only the decoded-inbox amendment to 0065. Correct its title, status and
false next-sequential-number explanation. Preserve its five numbered ruling
clauses verbatim, including the historical acceptance prerequisite. Retain
original proposed status as history and add a dated note explaining that the
operator accepted this exact ruling on 2026-10-24. Explicitly scope the original
“Proposed ruling”, design-only handoff and no-acceptance claims to the earlier
draft. Record the mistaken former number and all three reservations.

The preserved ruling selects the existing inbox segment before decoding it
exactly once with strict UTF-8. Success supplies the decoded request label;
invalid UTF-8 retains the entire original segment, including valid escapes
elsewhere. Preserve Unicode spelling, case, padding, literal plus, malformed
percent handling and encoded slash as data. `%2F` becomes `/` inside the label;
`%252F` becomes `%2F`. Valid aliases coalesce. Refused `%FF` and admitted
`%25FF` may both account under `%FF`; metrics prove neither admission nor storage.

All other route, extraction, validation, refusal, await and metric semantics
stand: middleware order, immutable guard key and RAII lifetimes, totals
including scrapes/refusals, family names/escaping, MCP identity, cumulative
4096-name cap and reserved LF overflow under
[0047 §2](../../docs/decisions/0047-storage-rpc.md). Invalid UTF-8 does not
register an await. There is no new atomic-scrape, durability or cancellation
promise. Preserve accept==stored, subscribe-before-scan, oldest-only pruning
and ingest extraction. Recording acceptance does not change source or deployed
behavior; runtime implementation and selector migration remain separate work.

## Panel synthesis and rulings

Adopt simplicity's single normative owner, verbatim clauses and eight-document
implementation boundary. Adopt robustness's explicit provenance, historical
comparisons, precise supersession scope and repair of owning references.
Both current positions agree on behavior and the bounded patch; no conflicting
semantic rules need averaging. Robustness's failure analysis constrains claims
about storage, webhook, chaos, TLS and parallel waits; it creates no work in
those systems for this task.

Rule for preserving all five clauses verbatim over a substance-only rewrite,
which robustness allowed: dated clarification is sufficient and gives reviewers
an exact comparison. The cost is retaining historical proposal language with
an adjacent explanation; rewriting it would lose the original acceptance
boundary. Rule against a rename/status-only patch: the extra reference edits
are necessary to remove contradictory current prerequisites. Their cost is
several small provenance edits, not additional normative copies.

Adopt simplicity's rejection of a new validator or amendment-test inventory
extension. Existing inventory/status mutation controls plus explicit semantic
and historical diff review are sufficient for this documentation task. The
cost is manual proof of the new 0065-to-0043 scope; existing finite amendment
tests do not prove it automatically. Reject broad renumbering, retrospective
history cleanup, decoding implementation, new fixtures and README selector
guidance here. Those alternatives either damage evidence or expand the task.

## Acceptance Criteria

1. **AC1 — One correctly identified record.** The decoded-inbox amendment
   exists only at `docs/decisions/0065-decoded-inbox-metrics.md`, is headed
   0065 and records `accepted (operator, 2026-10-24)`. Its dated provenance
   attributes the supplied chat ruling, corrects the former mistaken number
   and records #180=0062, #181=0063, #174=0064. No reserved lane is created,
   renumbered or otherwise changed; filesystem gaps do not release numbers.
2. **AC2 — Exact contract preservation.** All five numbered ruling clauses
   match the baseline verbatim. Alternatives, costs and proof obligations
   remain intact. Dated clarification explains the historical proposal and
   design-only language and says acceptance proves no runtime implementation
   or test result. No other decision is accepted.
3. **AC3 — Discovery and status agree.** Exactly one index row resolves to
   0065, identifies the narrow 0043 §6 amendment and matches its accepted
   status/date. No duplicate or old amendment row/link remains in affected
   current references. Existing decision inventory/status tests pass.
4. **AC4 — Narrow dated supersession.** Immediately after 0043 ruling clause
   6, add a 2026-10-24 accepted-0065 marker limiting the amendment to raw-path
   versus decoded-inbox request identity, with strict once-only decoding and
   whole-segment fallback. Add dated links at HTTP-plane spec Scope boundaries
   and plan Contracts and proof design AC3. Supersede only raw-key preservation,
   not all HTTP/await behavior. Preserve every original historical paragraph
   and existing 0047 marker byte-for-byte.
5. **AC5 — Owning references remain truthful.** In
   `specs/decoded-inbox-metrics/{spec,plan,tasks}.md`, retarget current links
   and decision identity to 0065 and add dated acceptance clarification.
   Explain obsolete present-tense prerequisites without erasing history.
   Preserve the entire 2026-10-24 validation receipt byte-for-byte, including
   the literal old filename and failed-gate account. Leave runtime tasks and
   mixed tasks with unexecuted proofs unchecked; satisfied acceptance alone
   does not complete T1 or the runtime feature.
6. **AC6 — Bounded diff.** Changes are limited to the eight logical existing
   documents listed in the plan and the three new design artifacts. No
   source, tests, dependencies, gates, policies, recipes, unrelated proposal,
   reserved-lane record or unrelated numbering reference changes. In
   particular, leave the unrelated 0065 mention in 0017 alone. No push, merge,
   issue closure or additional acceptance occurs.
7. **AC7 — Evidence and delivery.** Relative links in changed documentation
   resolve, exact historical/clauses comparisons and path review pass, and
   `cargo test --locked --test store_contract_docs` passes. Complete the
   current [house-rule gate inventory](../../docs/house-rules.md#gates--part-of-every-change),
   including pinned proofs and fail-closed native-tls absence; CI-only proofs
   remain CI-owned. Retain codex-fast engine triage, full design and independent
   Qwen Max correctness, Claude security, official DeepSeek Flash guarantees
   and GLM 5.3 tests review under [0057](../../docs/decisions/0057-codex-delivery-recipes.md).
   No Pro or fallback. A drafted design is not completed implementation,
   verification, review or delivery.
